package com.plugin.katagolocal

import android.app.Activity
import io.katago.android.engine.ExecuTorchEngine
import io.katago.android.engine.KataGoNet
import io.katago.android.engine.Mcts
import io.katago.android.engine.MctsMove
import java.io.File

/** GTP 좌표 문자 표기 - "I"를 건너뛴다(바둑/체스 표기 관례, KataGo GTP도 동일). */
private const val COLUMN_LETTERS = "ABCDEFGHJKLMNOPQRSTUVWXYZ"

private fun vertexToMove(vertex: String): MctsMove {
    if (vertex.equals("pass", ignoreCase = true)) return MctsMove.PASS
    val x = COLUMN_LETTERS.indexOf(vertex[0].uppercaseChar())
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
 * 패턴), genmove만 실제로 [Mcts.search]를 호출한다.
 *
 * 모델(.pte) 다운로드 흐름은 아직 없다(호스팅 위치 미정 - 나중에 결정) - 지금은
 * [MODEL_RELATIVE_PATH]에 파일을 직접 넣어두는 것(개발 중에는 adb push)을 전제로 한다.
 * 모델이 없으면 genmove가 "?"로 명확히 실패한다(GTP 프로토콜 규칙상 항상 응답은 해야
 * 하므로 예외를 던지지 않고 에러 문자열로 변환 - [handle] 참고).
 *
 * kata-analyze는 아직 구현하지 않았다(스트리밍이라 다른 설계가 필요 - 별도 단계).
 */
class GtpShell(private val activity: Activity) {
    private var boardSize = 19
    private var komi = 6.5

    // genmove가 Mcts.search()에 넘길 시뮬레이션 수 - GTP 표준 명령이 아니라 이 엔진만의
    // 확장(max_visits <n>). 실제 KataGo(원격 SSH)로 보내도 그냥 "? unknown command"로
    // 무시되니 무해함(commands::gtp::set_max_visits이 모든 연결된 세션에 best-effort로
    // 보냄 - set_komi와 동일한 패턴).
    private var maxVisits = DEFAULT_MAX_VISITS

    // (color, vertex) 쌍의 재생 순서 - undo가 마지막 것부터 지운다.
    private val history = mutableListOf<Pair<String, String>>()

    private var net: KataGoNet? = null

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
    fun handle(line: String): String {
        val tokens = line.trim().split(Regex("\\s+")).filter { it.isNotEmpty() }
        if (tokens.isEmpty()) return "? empty command"

        return try {
            when (tokens[0].lowercase()) {
                "protocol_version" -> "= 2"
                "name" -> "= katago-local (android_kata ExecuTorch engine)"
                "version" -> "= 0.1.0"
                "list_commands" ->
                    "= protocol_version\nname\nversion\nboardsize\nclear_board\nkomi\nplay\nundo\ngenmove\nmax_visits"
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

    private companion object {
        const val MODEL_RELATIVE_PATH = "models/tf3-b11c768.pte"
        const val DEFAULT_MAX_VISITS = 16
    }
}
