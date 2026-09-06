plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "io.katago.android.nativeboard"
    compileSdk = 36

    defaultConfig {
        minSdk = 26

        externalNativeBuild {
            cmake {
                cppFlags += "-std=c++17"
                arguments += "-DANDROID_STL=c++_shared"
            }
        }
        ndk {
            // KataGo's feature-encoding code (board.cpp/rules.cpp/nninputs.cpp) has no
            // architecture-specific code, so build for all four ABIs Play/most devices need.
            abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64", "x86")
        }
    }

    externalNativeBuild {
        cmake {
            path = file("src/main/cpp/CMakeLists.txt")
            // SDK cmake 패키지를 설치하지 않고 시스템 cmake를 쓴다 (local.properties의
            // cmake.dir 참고).
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}
