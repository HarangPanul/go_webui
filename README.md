# go_webui

Tauri v2 + Svelte 5 기반 모바일/데스크탑 앱. 원격 Linux 서버에 SSH로 접속해
KataGo(GTP)를 구동하고, 19x19 바둑판과 실시간 AI 분석을 렌더링한다.

자세한 설계/의사결정은 [PLAN.md](./PLAN.md) 참고.

## 디렉토리 구조

- `src/` — Svelte 5 프론트엔드 (바둑판, 설정 화면, 상태 스토어)
- `src-tauri/` — Rust 백엔드 (SSH 클라이언트, GTP 파이프라인, 커맨드)
- `static/` — 정적 에셋 (아이콘, 폰트 등)

## 개발 환경 (예정)

- Node.js + npm, Rust toolchain, Tauri CLI(`@tauri-apps/cli`)
- Android: Android SDK/NDK + `tauri android init`
- Desktop(Linux/Windows): 각 OS 네이티브 빌드 환경

```bash
npm install
npm run tauri dev
```
