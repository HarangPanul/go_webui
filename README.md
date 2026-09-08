# go_webui

Tauri v2 + Svelte 5 기반 모바일/데스크탑 앱. 원격 Linux 서버에 SSH로 접속해
KataGo(GTP)를 구동하거나(모든 플랫폼) Android에서는 온디바이스로 KataGo를 직접
구동해, 19x19 바둑판과 실시간 AI 분석을 렌더링한다.

자세한 설계/의사결정은 [PLAN.md](./PLAN.md) 참고.

## 디렉토리 구조

- `src/` — Svelte 5 프론트엔드 (바둑판, 설정 화면, 상태 스토어, `generated/bindings.ts`는 tauri-specta 자동 생성)
- `src-tauri/` — Rust 백엔드 (SSH 클라이언트, GTP 파이프라인/트랜스포트 추상화, services, 커맨드)
- `src-tauri/plugins/` — Android 온디바이스 KataGo 엔진 연동 플러그인 (`tauri-plugin-katago-local`, `android-engine`)
- `scripts/` — Android APK / Linux AppImage 빌드 스크립트
- `static/` — 정적 에셋 (아이콘, 폰트 등)

## 개발 환경

- Node.js + npm, Rust toolchain, Tauri CLI(`@tauri-apps/cli`)
- Linux 데스크탑: `webkit2gtk-4.1`, `base-devel`, `appmenu-gtk-module`, `libappindicator-gtk3`, `librsvg` 등 (Tauri v2 공식 요구사항)
- Android: Android SDK/NDK + `tauri android init` (온디바이스 엔진용 서브모듈은 `git submodule update --init --recursive` 필요)
- Desktop(Windows): 네이티브 빌드 환경 (아직 아이콘 `.ico` 미비 — PLAN.md 참고)

```bash
npm install
npm run tauri dev
```

### 빌드

```bash
npm run build       # 프론트엔드만
./scripts/build-appimage.sh   # Linux AppImage
./scripts/build-apk.sh        # Android APK (서명 포함)
```

### 테스트

```bash
npm run check   # svelte-check
npm test        # vitest
cargo test      # src-tauri (일부 통합 테스트는 실제 katago 필요해 #[ignore])
```
