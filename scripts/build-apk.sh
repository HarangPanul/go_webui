#!/usr/bin/env bash
# Android .apk 자동 빌드 스크립트.
# 사용법:
#   ./scripts/build-apk.sh            # release APK 빌드
#   ./scripts/build-apk.sh --debug    # debug APK 빌드
#
# 로컬 devDependency로 설치된 @tauri-apps/cli를 직접 실행하며 npx는 쓰지 않음.
# Android SDK/NDK 등 모바일 빌드 환경(ANDROID_HOME, NDK_HOME)이 이미 갖춰져 있어야 함.
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
ROOT_DIR="$(dirname -- "$SCRIPT_DIR")"
cd "$ROOT_DIR"

TAURI_BIN="$ROOT_DIR/node_modules/.bin/tauri"
if [[ ! -x "$TAURI_BIN" ]]; then
  echo "error: $TAURI_BIN 를 찾을 수 없습니다. 먼저 'npm install'을 실행하세요." >&2
  exit 1
fi

BUILD_MODE="release"
if [[ "${1:-}" == "--debug" ]]; then
  BUILD_MODE="debug"
fi

if [[ -z "${ANDROID_HOME:-}" && -z "${ANDROID_SDK_ROOT:-}" ]]; then
  echo "error: ANDROID_HOME(또는 ANDROID_SDK_ROOT) 환경 변수가 설정되어 있지 않습니다." >&2
  echo "       Android SDK/NDK 설치 후 환경 변수를 설정하세요 (NDK_HOME도 필요)." >&2
  exit 1
fi

# 아직 android 프로젝트가 생성된 적이 없으면 초기화
if [[ ! -d "$ROOT_DIR/src-tauri/gen/android" ]]; then
  echo "==> src-tauri/gen/android 가 없어 'tauri android init'을 먼저 실행합니다."
  "$TAURI_BIN" android init
fi

BUILD_ARGS=(android build --apk)
if [[ "$BUILD_MODE" == "debug" ]]; then
  BUILD_ARGS+=(--debug)
fi

# 빌드로 새로 생성/갱신된 apk만 골라내기 위한 시각 기준 마커
MARKER="$(mktemp)"
trap 'rm -f "$MARKER"' EXIT

echo "==> tauri ${BUILD_ARGS[*]} ($BUILD_MODE)"
"$TAURI_BIN" "${BUILD_ARGS[@]}"

APK_DIR="$ROOT_DIR/src-tauri/gen/android/app/build/outputs/apk"
echo
echo "==> 생성된 APK 파일:"
find "$APK_DIR" -name "*.apk" -newer "$MARKER" 2>/dev/null || find "$APK_DIR" -name "*.apk" 2>/dev/null
