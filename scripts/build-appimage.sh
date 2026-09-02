#!/usr/bin/env bash
# Linux AppImage 자동 빌드 스크립트.
# 사용법:
#   ./scripts/build-appimage.sh            # release AppImage 빌드
#   ./scripts/build-appimage.sh --debug    # debug AppImage 빌드
#
# 로컬 devDependency로 설치된 @tauri-apps/cli를 직접 실행하며 npx는 쓰지 않음.
# 프런트엔드 빌드(npm run build)는 tauri.conf.json의 beforeBuildCommand가 알아서
# 실행하므로 여기서 따로 호출하지 않음. AppImage 생성에 필요한 linuxdeploy/
# appimagetool은 tauri-bundler가 처음 실행 시 자동으로 내려받음(인터넷 연결 필요).
set -euo pipefail

# linuxdeploy/appimagetool 자체가 AppImage라서 실행하려면 FUSE가 필요한데,
# libfuse2 호환 shim 없이 fuse3만 깔린 환경(예: 이 저장소가 개발되는 컨테이너)에서는
# "dlopen(): error loading libfuse.so.2"로 바로 실패한다. AppImage 실행 시
# FUSE 마운트 대신 임시 디렉터리에 풀어서 바로 실행하도록 강제하면 FUSE 없이도
# 동작하므로 항상 켜 둠(정상적으로 libfuse2가 있는 환경에서도 무해함).
export APPIMAGE_EXTRACT_AND_RUN=1

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

BUILD_ARGS=(build --bundles appimage)
if [[ "$BUILD_MODE" == "debug" ]]; then
  BUILD_ARGS+=(--debug)
fi

# 빌드로 새로 생성/갱신된 AppImage만 골라내기 위한 시각 기준 마커
MARKER="$(mktemp)"
trap 'rm -f "$MARKER"' EXIT

echo "==> tauri ${BUILD_ARGS[*]} ($BUILD_MODE)"
"$TAURI_BIN" "${BUILD_ARGS[@]}"

BUNDLE_DIR="$ROOT_DIR/src-tauri/target/$BUILD_MODE/bundle/appimage"
echo
echo "==> 생성된 AppImage 파일:"
find "$BUNDLE_DIR" -name "*.AppImage" -newer "$MARKER" 2>/dev/null || find "$BUNDLE_DIR" -name "*.AppImage" 2>/dev/null
