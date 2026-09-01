package com.example.gowebui

import android.os.Bundle
import androidx.activity.enableEdgeToEdge
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    hideSystemBars()
  }

  // 상태 바/제스처 내비게이션 바를 "sticky immersive" 모드로 숨김: 평소엔 안
  // 보이다가 화면 가장자리를 스와이프하면 반투명하게 잠깐 나타났다 자동으로
  // 다시 숨겨짐(완전히 못 쓰게 막는 lean-back 모드는 아님).
  private fun hideSystemBars() {
    WindowCompat.setDecorFitsSystemWindows(window, false)
    val controller = WindowInsetsControllerCompat(window, window.decorView)
    controller.hide(WindowInsetsCompat.Type.systemBars())
    controller.systemBarsBehavior =
      WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
  }

  // 시스템 바는 앱이 포커스를 다시 얻을 때(다른 앱 전환 후 복귀, 키보드 닫힘 등)
  // 자동으로 다시 나타나므로, 포커스를 되찾을 때마다 다시 숨겨줘야 계속 유지된다.
  override fun onWindowFocusChanged(hasFocus: Boolean) {
    super.onWindowFocusChanged(hasFocus)
    if (hasFocus) {
      hideSystemBars()
    }
  }
}
