plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "io.katago.android.engine"
    compileSdk = 36

    defaultConfig {
        minSdk = 26
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    packaging {
        jniLibs {
            // engine-native (ANDROID_STL=c++_shared) and executorch-android's fbjni
            // dependency both bundle their own libc++_shared.so; AGP builds this module's
            // (empty) androidTest APK too, which needs this to merge cleanly.
            pickFirsts += "**/libc++_shared.so"
        }
    }
}

dependencies {
    api(project(":engine-native"))

    // docs/decisions/0002-litert-vs-executorch.md: ExecuTorch가 1차(유일) 백엔드.
    // 버전은 2026-09 기준 Maven Central 최신 안정 릴리스(1.1.0). python `executorch`
    // 패키지(1.4.1)보다 낮아 .pte 포맷 호환성을 실제 기기에서 재확인해야 한다.
    implementation("org.pytorch:executorch-android:1.1.0")
}
