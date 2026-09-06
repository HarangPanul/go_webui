cmake_minimum_required(VERSION 3.22)
project(katago_native CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# katago/ 는 android_kata 프로젝트(go_webui 기준 ~/dev/proj/android_kata)에서 이식해온
# git submodule(lightvector/KataGo @ v1.18.2)로, engine-native와 나란히
# plugins/android-engine/katago에 있다 - android_kata 원본은 repo root/android/
# engine-native/... 로 한 단계 더 중첩되어 있어 상위로 5단계(../../../../../)를
# 거슬러 올라갔지만, 여기서는 android-engine/ 바로 아래가 그 "루트"라 4단계면 된다.
get_filename_component(KATAGO_CPP_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../../../../katago/cpp" ABSOLUTE)

if(NOT EXISTS "${KATAGO_CPP_DIR}/game/board.cpp")
    message(FATAL_ERROR
        "katago submodule not found at ${KATAGO_CPP_DIR}. Run "
        "'git submodule update --init --recursive' from the repo root.")
endif()

add_library(katago_native SHARED
    jni_bridge.cpp
    ${KATAGO_CPP_DIR}/core/global.cpp
    ${KATAGO_CPP_DIR}/core/hash.cpp
    ${KATAGO_CPP_DIR}/core/rand.cpp
    ${KATAGO_CPP_DIR}/core/rand_helpers.cpp
    ${KATAGO_CPP_DIR}/core/test.cpp
    ${KATAGO_CPP_DIR}/core/sha2.cpp
    ${KATAGO_CPP_DIR}/core/md5.cpp
    ${KATAGO_CPP_DIR}/core/timer.cpp
    ${KATAGO_CPP_DIR}/core/bsearch.cpp
    ${KATAGO_CPP_DIR}/game/board.cpp
    ${KATAGO_CPP_DIR}/game/rules.cpp
    ${KATAGO_CPP_DIR}/game/boardhistory.cpp
    ${KATAGO_CPP_DIR}/neuralnet/nninputs.cpp
)

target_include_directories(katago_native PRIVATE ${KATAGO_CPP_DIR})

if(ANDROID)
    # core/sha2.cpp expects the BSD/glibc-style BYTE_ORDER/LITTLE_ENDIAN/BIG_ENDIAN macros
    # (normally pulled in transitively via <sys/types.h> on desktop libc's), which bionic
    # doesn't define the same way. All Android ABIs we target (arm64-v8a, armeabi-v7a,
    # x86_64, x86) are little-endian, so this is a safe, faithful substitute.
    target_compile_definitions(katago_native PRIVATE
        LITTLE_ENDIAN=1234
        BIG_ENDIAN=4321
        BYTE_ORDER=1234
    )
endif()

find_library(log-lib log)
target_link_libraries(katago_native ${log-lib})
