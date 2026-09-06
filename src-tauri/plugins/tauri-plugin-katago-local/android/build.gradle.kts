plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
}

android {
    compileSdk = 36
    namespace = "com.plugin.katagolocal"
    defaultConfig {
        // targetSdk는 라이브러리 모듈 DSL에서 곧 제거될 예정이라(AGP 경고) 뺐다 - 실제
        // targetSdk는 이 모듈을 가져다 쓰는 :app(gen/android/app/build.gradle.kts)이
        // 정의한 값을 그대로 따른다.
        //
        // 26: android-engine/engine, engine-native가 minSdk 26을 요구함(android_kata에서
        // 그대로 이식) - 앱 전체(:app)도 함께 24 -> 26으로 올렸다.
        minSdk = 26
    }
    // 메인 앱(gen/android/app/build.gradle.kts)과 동일하게 맞춰야 함 - 이게 없으면
    // javac(기본 1.8)와 kotlinc(기본 17) 사이의 JVM-target 불일치로 compileDebugKotlin이
    // 실패한다("Inconsistent JVM-target compatibility").
    kotlinOptions {
        jvmTarget = "1.8"
    }
    lint {
        abortOnError = false
    }
}

dependencies {
    implementation(project(":tauri-android"))
    // android_kata에서 이식한 온디바이스 KataGo 추론 엔진(KataGoNet/ExecuTorchEngine/
    // Mcts/NativeBoard) - GtpShell.kt가 실제 탐색에 사용.
    implementation(project(":engine"))
}
