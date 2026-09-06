package com.plugin.katagolocal

import android.app.Activity
import android.os.Process
import android.util.Log
import app.tauri.plugin.Channel
import io.katago.android.engine.ExecuTorchEngine
import io.katago.android.engine.KataGoNet
import io.katago.android.engine.Mcts
import io.katago.android.engine.MctsMove
import java.io.File
import java.util.Locale
import kotlin.math.tanh

/** GTP 좌표 문자 표기 - "I"를 건너뛴다(바둑/체스 표기 관례, KataGo GTP도 동일). */
private const val COLUMN_LETTERS = "ABCDEFGHJKLMNOPQRSTUVWXYZ"

private fun vertexToMove(vertex: String): MctsMove {
    if (vertex.equals("pass", ignoreCase = true)) return MctsMove.PASS
    val x = COLUMN_LETTERS.indexOf(vertex[0].uppercaseChar())
    // COLUMN_LETTERS.indexOf는 못 찾으면 -1을 돌려주는데, 그걸 그대로 넘기면
    // MctsMove(x, y).isPass가 "x < 0 || y < 0"라 알아채지 못한 채 조용히 PASS로
    // 둔갑해버린다(예: 잘못된 열 글자가 들어와도 에러 없이 그냥 지나감) - 지금은
    // Rust coords.rs가 항상 같은 글자 집합("I" 제외)으로만 vertex를 만들어 실제로
    // 발생하지 않지만, 그 불변식이 깨지면(예: GTP 콘솔로 손으로 잘못된 명령을 보냄)
    // 이 없이는 원인을 알기 어려운 엉뚱한 수로 이어진다.
    require(x >= 0) { "invalid GTP vertex column: $vertex" }
    val row = vertex.substring(1).toInt()
    return MctsMove(x, row - 1)
}

private fun moveToVertex(move: MctsMove): String {
    if (move.isPass) return "pass"
    return "${COLUMN_LETTERS[move.x]}${move.y + 1}"
}

/**
 * GTP 명령 해석기 - android_kata에서 이식한 NativeBoard/KataGoNet(ExecuTorchEngine)/Mcts로
 * 실제 온디바이스 추론을 수행한다. boardsize/clear_board/komi/play/undo는 상태만 반영하고
 * (NativeBoard는 undo 원시 기능이 없어 매번 처음부터 재생 - Mcts 자신의 설계와 동일한
 * 패턴), genmove와 kata-analyze만 실제로 [Mcts]를 돌린다.
 *
 * 모델(.pte) 다운로드 흐름은 아직 없다(호스팅 위치 미정 - 나중에 결정) - 지금은
 * [MODEL_RELATIVE_PATH]에 파일을 직접 넣어두는 것(개발 중에는 adb push)을 전제로 한다.
 * 모델이 없으면 genmove/kata-analyze가 "?"로 명확히 실패한다(GTP 프로토콜 규칙상 항상
 * 응답은 해야 하므로 예외를 던지지 않고 에러 문자열로 변환 - [handle] 참고).
 *
 * ## kata-analyze 스트리밍
 *
 * 일반 명령(genmove 등)은 gtpLine 호출 하나에 응답 하나로 끝나지만, kata-analyze는
 * "다른 명령이 올 때까지 계속 info 줄을 내보내는" 스트리밍 명령이라 같은 방식으로 표현할
 * 수 없다. 그래서:
 * - [handle]은 kata-analyze를 받으면 [startAnalysis]로 백그라운드 스레드만 띄우고 즉시
 *   "= "로 응답한다(이 호출 자체가 오래 블록될 필요는 없음 - GtpSession 쪽에서 이 응답이
 *   빨리 오든 늦게 오든 상관없이 info 줄은 어차피 별도 채널로 옴).
 * - 이후 info 줄들은 그 호출이 실어온 [Channel]로, 원래 호출과 무관한 시점에 계속
 *   [Channel.sendObject]된다(android_transport.rs가 이 채널 데이터를 GtpSession이
 *   기대하는 TransportEvent::Line으로 그대로 흘려보냄 - GtpSession 자신은 이게 어느
 *   경로로 왔는지 전혀 모른다).
 * - 실제 GTP 프로토콜은 "새 입력이 오면 스트리밍이 멈춘다"인데, 우리는 stdin 한 줄
 *   읽기가 아니라 명시적 호출이므로 그 대신 [handle]이 매 호출 맨 앞에서 무조건
 *   [stopAnalysis]를 불러 이전 스트림을 끊는다(연속 호출로 자연히 인터럽트되는 것과
 *   동일한 효과 - GameControls.svelte가 보드 위치 바뀔 때마다 kata-analyze를 재호출하는
 *   패턴과 맞물려 자연스럽게 "새 위치로 재시작"이 된다).
 * - ownership은 실제 KataGo처럼 탐색 트리 전체의 방문 가중 평균이 아니라, 루트 위치를
 *   신경망이 한 번 평가한 원본 값 그대로(Mcts.rootOwnership 참고) - 최소 구현으로서의
 *   근사이지 진짜 KataGo와 수치가 정확히 같지는 않다.
 */
class GtpShell(private val activity: Activity) {
    private var boardSize = 19
    private var komi = 6.5

    // genmove가 Mcts.search()에 넘길 시뮬레이션 수 - GTP 표준 명령이 아니라 이 엔진만의
    // 확장(max_visits <n>). 실제 KataGo(원격 SSH)로 보내도 그냥 "? unknown command"로
    // 무시되니 무해함(commands::local_engine::set_max_visits이 모든 연결된 세션에
    // best-effort로 보냄 - set_komi와 동일한 패턴).
    private var maxVisits = DEFAULT_MAX_VISITS

    // (color, vertex) 쌍의 재생 순서 - undo가 마지막 것부터 지운다.
    private val history = mutableListOf<Pair<String, String>>()

    private var net: KataGoNet? = null

    // kata-analyze 스트리밍 루프. handle()이 매 호출 맨 앞에서 stopAnalysis()로 정리하기
    // 때문에, 이 스레드가 실행 중인 동안에는 handle() 본문(history/boardSize/net을 만지는
    // 부분)이 절대 동시에 실행되지 않는다 - 즉 이 필드에 대한 접근은 handle()을 호출하는
    // 단일 executor 스레드(KatagoLocalPlugin.worker)에서만 일어나므로 별도 동기화가
    // 필요 없다.
    private var analysisThread: Thread? = null

    // 내부 저장소(filesDir)를 쓴다 - getExternalFilesDir()/Android/data/<pkg>/files는
    // adb push로 넣은 파일이 앱 자신의 프로세스에서는 안 보이는 경우가 실제로 있다(FUSE
    // 기반 scoped storage 에뮬레이션 캐시 문제 - `run-as`로는 보여도 앱 프로세스 자체
    // 마운트 네임스페이스에서는 갱신 전일 수 있음). 내부 저장소는 이런 문제가 없다.
    private fun modelFile(): File = File(activity.filesDir, MODEL_RELATIVE_PATH)

    /** 이미 로드했으면 그대로 재사용 - 매 genmove마다 .pte를 다시 여는 건 낭비. */
    private fun ensureNet(): KataGoNet {
        net?.let { return it }
        val file = modelFile()
        check(file.exists()) {
            "model not found at ${file.absolutePath} - 아직 실행 중 자동 다운로드가 없음, " +
                "지금은 adb push로 직접 넣어야 함(호스팅 위치 결정 후 자동화 예정)"
        }
        return ExecuTorchEngine(file.absolutePath, boardSize, boardSize).also { net = it }
    }

    @Synchronized
    fun handle(line: String, channel: Channel): String {
        // 실제 GTP는 "새 입력이 오면 스트리밍이 멈춘다"는 규칙으로 kata-analyze를
        // 인터럽트한다 - 우리는 stdin이 아니라 매 호출이 독립적이므로, 그 규칙을 "다음
        // 어떤 명령이 오든 이전 스트림부터 정리한다"로 그대로 재현한다(클래스 kdoc
        // 참고). stopAnalysis()는 인터럽트 신호만 보내고 기다리지 않는다(그 이유는
        // stopAnalysis 자신의 kdoc 참고) - history/boardSize는 이 스레드가 아예 만지지
        // 않으므로 바로 이어서 안전하게 고칠 수 있다.
        stopAnalysis()

        val tokens = line.trim().split(Regex("\\s+")).filter { it.isNotEmpty() }
        if (tokens.isEmpty()) return "? empty command"

        return try {
            when (tokens[0].lowercase()) {
                "protocol_version" -> "= 2"
                "name" -> "= katago-local (android_kata ExecuTorch engine)"
                "version" -> "= 0.1.0"
                "list_commands" ->
                    "= protocol_version\nname\nversion\nboardsize\nclear_board\nkomi\nplay\nundo\ngenmove\nmax_visits\nkata-analyze"
                "boardsize" -> {
                    val newSize = tokens[1].toInt()
                    if (newSize != boardSize) {
                        // 보드 크기가 바뀌면 지금 로드된 net(nnXLen/nnYLen이 그 크기로
                        // 고정됨)을 그대로 쓸 수 없다 - 다음 genmove에서 새 크기로 다시
                        // 로드하도록 비워둔다.
                        net?.close()
                        net = null
                    }
                    boardSize = newSize
                    history.clear()
                    "= "
                }
                "clear_board" -> {
                    history.clear()
                    "= "
                }
                "komi" -> {
                    komi = tokens[1].toDouble()
                    "= "
                }
                "max_visits" -> {
                    val n = tokens[1].toInt()
                    require(n > 0) { "max_visits must be positive, got $n" }
                    maxVisits = n
                    "= "
                }
                "play" -> {
                    history.add(tokens[1] to tokens[2])
                    "= "
                }
                "undo" -> {
                    if (history.isEmpty()) {
                        "? cannot undo"
                    } else {
                        history.removeAt(history.size - 1)
                        "= "
                    }
                }
                "genmove" -> {
                    val vertex = moveToVertex(genmoveReal())
                    history.add(tokens[1] to vertex)
                    "= $vertex"
                }
                "kata-analyze" -> {
                    // 두 번째 토큰은 간격(centisecond). 그 뒤 "ownership true" 같은
                    // 옵션 플래그는 파싱하지 않고 무시 - 지금은 항상 ownership을 함께
                    // 보고한다(우리 유일한 호출자인 start_kata_analyze가 항상
                    // "ownership true"를 붙여 보내므로 실질적으로 차이가 없음).
                    val intervalCentiseconds = tokens.getOrNull(1)?.toLongOrNull()
                    require(intervalCentiseconds != null && intervalCentiseconds > 0) {
                        "kata-analyze requires a positive interval, got ${tokens.getOrNull(1)}"
                    }
                    startAnalysis(intervalCentiseconds, channel)
                    "= "
                }
                else -> "? unknown command"
            }
        } catch (e: Exception) {
            "? ${e.message ?: e.javaClass.simpleName}"
        }
    }

    /**
     * 지금까지의 [history]를 Mcts의 rootHistory로 그대로 넘겨 그 위치에서부터
     * 탐색한다(Mcts.kt의 go_webui local patch 참고 - 원래 android_kata 버전은 항상 빈
     * 보드에서만 탐색 가능했음). [maxVisits]는 기본 16(android_kata 벤치마크 기준
     * 시뮬레이션당 ~0.6초라 최악의 경우 모델 로드(~1초) 포함 10초 안팎) - `max_visits`
     * GTP 확장 명령으로 실행 중에 바꿀 수 있다.
     */
    private fun genmoveReal(): MctsMove {
        val net = ensureNet()
        val rootHistory = history.map { (_, vertex) -> vertexToMove(vertex) }
        val mcts = Mcts(net, boardSize, rootHistory = rootHistory)
        return mcts.search(maxVisits)
    }

    /**
     * 이미 실행 중인 스트림이 있으면 인터럽트 신호만 보내고 끝난다 - **기다리지
     * 않는다(더 이상 join() 하지 않음)**.
     *
     * 예전엔 여기서 `thread.join()`까지 했었다: 한 시뮬레이션이 진행 중이면(기기에서
     * 시뮬레이션 하나가 이미 수백ms~1초 걸릴 수 있음 - genmoveReal 문서 참고) 그게
     * 끝날 때까지 이 함수가 막혔는데, [handle]이 단일 executor 스레드(worker)에서
     * `@Synchronized`로 불리는 구조상 이 막힘이 곧 "이 플러그인 전체가 그만큼 멈춤"과
     * 같았다 - 실기기에서(장시간 분석으로 인한 발열 스로틀링, 백그라운드 스레드
     * 우선순위 저하 등) 시뮬레이션 한 번이 예상보다 훨씬 오래 걸리는 경우가 실제로
     * 있었고, 그동안은 착수(play)/genmove/kata-analyze 재시작 전부가 한꺼번에
     * "멈춘 것처럼" 보였다(Analysis/Ownership을 껐다 켜면 돌아오는 것처럼 보인 건
     * 실제로 고쳐져서가 아니라, 끄는 동작 자체가 부하를 없애 발열이 식을 시간을 벌어준
     * 것뿐이었다).
     *
     * join 없이도 안전한 이유: 이 백그라운드 스레드가 만지는 공유 가변 상태는
     * [net](KataGoNet) 하나뿐이다(history/boardSize/route 자체는 이 스레드가 전혀
     * 건드리지 않고, mcts/engine은 시작할 때 로컬 변수로만 캡처해 씀) - 그리고
     * net.evaluate()/close()는 이제 ExecuTorchEngine 내부에서 서로 배타적으로
     * 실행되도록 synchronized돼 있어(ExecuTorchEngine.kt 참고), 이 스레드가 인터럽트를
     * 아직 못 보고 진행 중인 마지막 한 번의 추론과 그 뒤 [handle]이 곧장 이어서 하는
     * `net.close()`/재할당이 서로 겹쳐도 순서만 보장되면 충돌하지 않는다. 다만 그
     * 마지막 한두 번의 "info" 줄이 새 위치가 아닌 옛 위치 기준으로 늦게 나갈 수는
     * 있다 - process.rs::AnalysisContext kdoc에 이미 문서화된 것과 같은 종류의 경합이고,
     * 다음 갱신 주기(수백ms) 안에 새 데이터로 덮어써진다.
     */
    private fun stopAnalysis() {
        analysisThread?.interrupt()
        analysisThread = null
    }

    /**
     * 지금 위치에서부터 백그라운드 스레드로 계속 시뮬레이션을 쌓아가며(같은 [Mcts]
     * 인스턴스에 반복해서 search()를 호출 - 트리는 매번 이어짐, 새로 안 만듦) 매
     * 보고 주기마다 root의 현재 상태를 "info ..." 줄로 [channel]에 흘려보낸다.
     * [stopAnalysis]로 인터럽트될 때까지 멈추지 않는다(genmove처럼 정해진 시뮬레이션
     * 수가 없음 - 실제 KataGo의 kata-analyze와 동일).
     *
     * 한 배치에 시뮬레이션 하나만 돌린다 - 기기에서 시뮬레이션 하나가 이미 요청 간격보다
     * 오래 걸릴 수 있어서(genmoveReal 문서의 ~0.6초/시뮬레이션 참고) 여러 개를 묶어
     * 돌리면 응답성이 요청한 interval보다 훨씬 나빠진다. 대신 매 시뮬레이션 직후
     * 보고하고, 남은 시간이 있을 때만 그만큼 잠들어 요청한 간격을 "최소 간격"으로
     * 지킨다.
     *
     * 스레드 우선순위를 [Process.THREAD_PRIORITY_BACKGROUND]로 낮춰서 시작한다 -
     * 이 스레드는 계속(사람이 분석을 켜둔 동안 끊임없이) 신경망 추론을 돌리는데,
     * [handle]을 부르는 executor 스레드(사람의 실제 착수/genmove)와 우선순위가 같으면
     * OS 스케줄러가 둘을 동등하게 취급해 서로의 CPU 시간을 갉아먹는다 - 특히 그 경합이
     * 바로 "간헐적으로 시뮬레이션 한 번이 예상보다 훨씬 오래 걸림"(stopAnalysis kdoc이
     * 설명하는, 예전엔 이게 전체 멈춤으로 번졌던 그 증상)의 유력한 원인 중 하나다.
     * 분석은 배경 작업이라 실제 착수 처리보다 늦어져도 되므로, 낮춰서 항상 후순위로
     * 밀리게 하는 편이 맞다.
     */
    private fun startAnalysis(intervalCentiseconds: Long, channel: Channel) {
        val engine = ensureNet()
        val rootHistory = history.map { (_, vertex) -> vertexToMove(vertex) }
        val mcts = Mcts(engine, boardSize, rootHistory = rootHistory)
        val intervalMillis = intervalCentiseconds * 10

        val thread = Thread {
            Process.setThreadPriority(Process.THREAD_PRIORITY_BACKGROUND)
            try {
                while (!Thread.currentThread().isInterrupted) {
                    val tickStart = System.currentTimeMillis()

                    mcts.search(1)
                    if (Thread.currentThread().isInterrupted) break

                    formatAnalysisLine(mcts, boardSize)?.let { channel.sendObject(it) }

                    val remaining = intervalMillis - (System.currentTimeMillis() - tickStart)
                    if (remaining > 0) Thread.sleep(remaining)
                }
            } catch (e: InterruptedException) {
                // stopAnalysis()의 interrupt() - 정상 종료 경로.
            } catch (e: Exception) {
                // mcts.search()가 던질 수 있는 그 외 모든 실패(예: evaluatePath의
                // "illegal move replayed" check, 네이티브 ExecuTorch 추론 오류) -
                // 잡아두지 않으면 스레드가 조용히 죽어서(기본 uncaught exception
                // handler가 로그를 남기더라도 이 클래스와 무관한 형태로) 사용자
                // 입장에선 "분석이 그냥 멈췄다"는 것 말고는 원인을 알 방법이 없다.
                // 최소한 어디서 왜 멈췄는지는 logcat에 남긴다 - 별도 채널로
                // 프런트에 에러를 알리는 프로토콜은 아직 없음(GTP kata-analyze
                // 자체에 스트림 중간 에러를 표현하는 표준 라인이 없어서).
                Log.e("GtpShell", "kata-analyze stream stopped unexpectedly", e)
            }
        }
        thread.isDaemon = true
        analysisThread = thread
        thread.start()
    }

    private companion object {
        const val MODEL_RELATIVE_PATH = "models/tf3-b11c768.pte"
        const val DEFAULT_MAX_VISITS = 16

        /**
         * Mcts의 현재 root 상태를 하나의 "info move ... info move ... ownership ..."
         * 줄로 직렬화한다 - go_webui의 gtp::parser::parse_kata_analyze가 기대하는
         * 정확히 그 형식(각 move 블록은 "move/visits/winrate/scoreLead/pv"를 이 순서일
         * 필요 없이 포함하면 되고, pv는 최소 한 좌표만 있어도 파싱됨 - 우리는 실제
         * 다중 수순을 추적하지 않으므로 그 수 하나만 pv로 보고한다). 아직 한 번도
         * 시뮬레이션이 안 끝나 후보가 없으면(이론상 발생 안 함 - search(1)을 먼저
         * 부른 뒤에만 호출됨) null.
         *
         * String.format에 항상 Locale.ROOT를 명시한다 - 기기 로케일에 따라 소수점이
         * ','로 나오는 등 GTP 파서가 못 읽는 형식이 될 수 있어서(실제로 일부 로케일이
         * 그렇게 동작함 - 흔한 Android 함정).
         */
        fun formatAnalysisLine(mcts: Mcts, boardSize: Int): String? {
            val candidates = mcts.rootCandidates()
            if (candidates.isEmpty()) return null

            // 실제 KataGo는 후보마다 각자의 score 추정치를 보고하지만, 우리 최소
            // 탐색은 수별 score를 따로 추적하지 않는다(승률만 backup) - 대신 루트
            // ownership에서 유도한 보드 전체 점수 추정 하나를 모든 후보에 공통으로
            // 보고한다(Mcts.scoreUtilityValue의 scoreEstimate와 같은 근사 - 다만 거긴
            // 탐색용 utility로 쓰려고 정규화/outer tanh까지 거치는 반면, 여기서는
            // "몇 집 차이"로 보여줘야 하므로 그 정규화는 하지 않고 원 스케일 그대로 씀).
            val rootOwnership = mcts.rootOwnership()
            val scoreLead = rootOwnership?.sumOf { tanh(it.toDouble()) } ?: 0.0

            val sb = StringBuilder()
            for (c in candidates) {
                val vertex = moveToVertex(c.move)
                sb.append("info move ")
                sb.append(vertex)
                sb.append(" visits ")
                sb.append(c.visits)
                sb.append(" winrate ")
                sb.append(String.format(Locale.ROOT, "%.4f", c.winrateForRootMover))
                sb.append(" scoreLead ")
                sb.append(String.format(Locale.ROOT, "%.2f", scoreLead))
                sb.append(" pv ")
                sb.append(vertex)
                sb.append(' ')
            }

            if (rootOwnership != null) {
                sb.append("ownership")
                // rootOwnership의 인덱스는 신경망 텐서 좌표(모델 y=0 == GTP 행 1 ==
                // 보드 "아래쪽" 줄 - GtpShell의 expand()/moveToVertex와 동일한 규칙).
                // 그런데 go_webui 프런트(OwnershipOverlay.svelte/coords.ts)가 기대하는
                // ownership 배열은 "위쪽 줄부터" 나열된 row-major(로컬 y=0 == 보드
                // "위쪽" 줄) - 그래서 행 순서를 뒤집어서(localY -> modelY = size-1-localY)
                // 출력해야 실제 SSH KataGo가 내는 것과 같은 순서가 된다.
                for (localY in 0 until boardSize) {
                    val modelY = boardSize - 1 - localY
                    for (x in 0 until boardSize) {
                        sb.append(' ')
                        sb.append(String.format(Locale.ROOT, "%.4f", tanh(rootOwnership[modelY * boardSize + x].toDouble())))
                    }
                }
            }

            return sb.toString().trim()
        }
    }
}
