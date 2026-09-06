package com.plugin.katagolocal

import android.app.Activity
import android.util.Log
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Channel
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.util.concurrent.Executors

@InvokeArg
class PingArgs {
    lateinit var value: String
}

@InvokeArg
class GtpLineArgs {
    lateinit var line: String

    // kata-analyze 스트리밍("info ..." 라인)을 이 호출의 반환(response)과 별도로
    // 비동기로 밀어넣는 통로 - GtpShell.handle()이 kata-analyze 실행 중에는 이 채널을
    // 붙잡아두고 몇 번이고 sendObject()를 부른다. 모든 gtpLine 호출에 항상 실려 오지만
    // (android_transport.rs::open()이 세션당 하나만 만들어 매번 그대로 실어 보냄)
    // kata-analyze가 아닌 명령에서는 그냥 쓰이지 않는다.
    lateinit var channel: Channel
}

// ping은 Rust <-> Kotlin 플러그인 배선(Cargo 경로 의존성 -> gen/android 자동 Gradle
// 등록 -> JNI 왕복) 자체가 동작하는지 확인용으로 남겨둔 스텁. 실제 GTP 왕복은 gtpLine이
// 담당하고, 명령 해석/실제 추론은 GtpShell(android_kata의 KataGoNet/Mcts/NativeBoard 사용)에
// 있다.
@TauriPlugin
class KatagoLocalPlugin(private val activity: Activity) : Plugin(activity) {
    private val gtpShell = GtpShell(activity)

    // Tauri의 플러그인 커맨드 디스패치(PluginManager.runCommand)는 Android 메인/UI
    // 스레드에서 호출된다(wry의 run_on_android_context가 앱의 메인 Looper에 붙어
    // 있음) - genmove는 Mcts.search()가 신경망 추론을 여러 번 돌려 수 초씩 걸릴 수
    // 있으므로(android_kata 벤치마크 기준 시뮬레이션당 ~0.6초) 그 자리에서 처리하면
    // UI가 멈추고 ANR로 이어진다. 단일 스레드 executor에 넘기면 GTP가 원래 요구하는
    // 순서 보장(FIFO)도 자연히 유지되면서 호출 스레드는 즉시 풀려난다.
    private val worker = Executors.newSingleThreadExecutor()

    @Command
    fun ping(invoke: Invoke) {
        val args = invoke.parseArgs(PingArgs::class.java)
        Log.i("KatagoLocalPlugin", "ping received value=${args.value}")
        val ret = JSObject()
        ret.put("value", args.value)
        invoke.resolve(ret)
    }

    @Command
    fun gtpLine(invoke: Invoke) {
        val args = invoke.parseArgs(GtpLineArgs::class.java)
        worker.execute {
            val response = gtpShell.handle(args.line, args.channel)
            Log.i("KatagoLocalPlugin", "gtp: ${args.line} -> $response")
            val ret = JSObject()
            ret.put("response", response)
            invoke.resolve(ret)
        }
    }
}
