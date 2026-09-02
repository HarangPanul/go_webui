#!/usr/bin/env bash
# Android .apk 자동 빌드 + 서명 스크립트.
# 사용법:
#   ./scripts/build-apk.sh            # release APK 빌드 + 서명
#   ./scripts/build-apk.sh --debug    # debug APK 빌드(이미 자동 서명되어 있음)
#
# 로컬 devDependency로 설치된 @tauri-apps/cli를 직접 실행하며 npx는 쓰지 않음.
# Android SDK/NDK 등 모바일 빌드 환경(ANDROID_HOME, NDK_HOME)이 이미 갖춰져 있어야 함.
#
# release 빌드는 별도 서명 설정이 없으면 Gradle이 "*-unsigned.apk"만 내보내고 끝나서
# 그 상태로는 기기에 설치할 수 없다. 이 스크립트가 apksigner로 직접 서명까지 마쳐서
# "*-signed.apk"로 내보낸다(기본은 로컬 debug keystore로 서명 - 각자 자기 기기에서
# 테스트하는 용도로는 충분하고, 그 키로 이미 설치된 debug 빌드 위에 그대로 덮어설치할
# 수도 있음). debug 빌드는 Android Gradle Plugin이 빌드 시점에 이미 debug keystore로
# 자동 서명해서 내보내므로 이 스크립트가 따로 손댈 필요가 없다.
#
# 진짜 release 키로 서명하고 싶으면 아래 환경 변수로 keystore를 바꿔 끼우면 된다:
#   ANDROID_KEYSTORE=/path/to/release.keystore
#   ANDROID_KEYSTORE_PASS=...
#   ANDROID_KEY_ALIAS=...
#   ANDROID_KEY_PASS=...        (생략하면 ANDROID_KEYSTORE_PASS와 동일하게 취급)
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
mapfile -t BUILT_APKS < <(find "$APK_DIR" -name "*.apk" -newer "$MARKER" 2>/dev/null)
if [[ ${#BUILT_APKS[@]} -eq 0 ]]; then
  mapfile -t BUILT_APKS < <(find "$APK_DIR" -name "*.apk" 2>/dev/null)
fi

if [[ ${#BUILT_APKS[@]} -eq 0 ]]; then
  echo "error: 빌드된 apk를 $APK_DIR 밑에서 찾지 못했습니다." >&2
  exit 1
fi

if [[ "$BUILD_MODE" == "debug" ]]; then
  echo
  echo "==> 생성된 APK 파일(debug - 이미 자동 서명됨):"
  printf '%s\n' "${BUILT_APKS[@]}"
  exit 0
fi

# --- 여기부터는 release 빌드 서명 ---

# build-tools 안의 apksigner를 찾는다(버전 디렉터리가 여러 개일 수 있어 가장 최신을 씀).
SDK_ROOT="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"
APKSIGNER=""
LATEST_BUILD_TOOLS="$(ls -d "$SDK_ROOT"/build-tools/*/ 2>/dev/null | sort -V | tail -n1)"
if [[ -n "$LATEST_BUILD_TOOLS" && -x "${LATEST_BUILD_TOOLS}apksigner" ]]; then
  APKSIGNER="${LATEST_BUILD_TOOLS}apksigner"
fi
if [[ -z "$APKSIGNER" ]]; then
  echo "error: apksigner를 찾을 수 없습니다 ($SDK_ROOT/build-tools/<version>/apksigner)." >&2
  echo "       Android SDK build-tools가 설치되어 있는지 확인하세요." >&2
  exit 1
fi

KEYSTORE_PATH="${ANDROID_KEYSTORE:-$HOME/.android/debug.keystore}"
KEYSTORE_PASS="${ANDROID_KEYSTORE_PASS:-android}"
KEY_ALIAS="${ANDROID_KEY_ALIAS:-androiddebugkey}"
KEY_PASS="${ANDROID_KEY_PASS:-$KEYSTORE_PASS}"

# 기본값(로컬 debug keystore)을 쓰는데 아직 파일이 없으면(한 번도 Android 빌드를 해본
# 적 없는 새 머신 등) Android가 쓰는 것과 똑같은 방식으로 새로 만들어준다. 사용자가
# ANDROID_KEYSTORE로 직접 다른 keystore를 지정했는데 그 파일이 없으면 오타일 가능성이
# 높으므로 대신 만들어주지 않고 에러로 멈춘다.
if [[ ! -f "$KEYSTORE_PATH" ]]; then
  if [[ -z "${ANDROID_KEYSTORE:-}" ]]; then
    echo "==> $KEYSTORE_PATH 가 없어 새로 생성합니다(Android 기본 debug keystore와 동일한 값)."
    mkdir -p "$(dirname "$KEYSTORE_PATH")"
    keytool -genkeypair -v \
      -keystore "$KEYSTORE_PATH" \
      -storepass "$KEYSTORE_PASS" \
      -keypass "$KEY_PASS" \
      -alias "$KEY_ALIAS" \
      -keyalg RSA -keysize 2048 -validity 10000 \
      -dname "CN=Android Debug,O=Android,C=US"
  else
    echo "error: ANDROID_KEYSTORE로 지정한 keystore 파일을 찾을 수 없습니다: $KEYSTORE_PATH" >&2
    exit 1
  fi
fi

echo
echo "==> 서명 완료된 APK 파일:"
for unsigned in "${BUILT_APKS[@]}"; do
  case "$unsigned" in
  *-unsigned.apk)
    signed="${unsigned%-unsigned.apk}-signed.apk"
    "$APKSIGNER" sign \
      --ks "$KEYSTORE_PATH" \
      --ks-pass "pass:$KEYSTORE_PASS" \
      --key-pass "pass:$KEY_PASS" \
      --ks-key-alias "$KEY_ALIAS" \
      --out "$signed" \
      "$unsigned"
    "$APKSIGNER" verify "$signed"
    echo "$signed"
    ;;
  *)
    # 이미 서명된 파일이 나온 경우(예: signingConfig가 gradle 쪽에 별도로 설정돼
    # 있는 경우) 그대로 둔다.
    echo "$unsigned"
    ;;
  esac
done
