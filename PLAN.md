### **1. 프로젝트 핵심 아키텍처**

```
┌─────────────────────────────────────────────────────────────┐
│                      Svelte 5 UI Layer                      │
│  - Canvas 기반 19x19 바둑판 & 터치 2단계 착수 UI            │
│  - kata-analyze 실시간 승률 / PV 변화도 오버레이            │
│  - SSH Key 텍스트 입력 및 설정을 위한 모바일 최적화 화면    │
└──────────────────────────────┬──────────────────────────────┘
                               │ Tauri IPC (Commands / Events)
┌──────────────────────────────▼──────────────────────────────┐
│                    Tauri v2 (Rust Core)                     │
│  - 앱 내부 샌드박스 저장소 (SSH Key 텍스트 안전 보관)       │
│  - Rust 네이티브 SSH Client (russh) 세션 유지                │
│  - Line-by-Line GTP/kata-analyze 스트리밍 파서              │
└──────────────────────────────┬──────────────────────────────┘
                               │ SSH Protocol (TCP / Port 22)
┌──────────────────────────────▼──────────────────────────────┐
│                      Remote Linux Server                    │
│             (KataGo Engine Process / GTP Interface)         │
└─────────────────────────────────────────────────────────────┘

```

---

### **2. 핵심 모듈별 상세 설계**

**가. 프론트엔드 (Svelte 5 + Tailwind CSS)**

* **바둑판 렌더링 Engine (Canvas API)**:
* HTML5 Canvas 레이어링 (배경 $\rightarrow$ 실제 돌 $\rightarrow$ AI 추천 수 $\rightarrow$ **임시 하이라이트**)
* 터치 이벤트 처리: 바둑판 터치 시 해당 교차점을 지나는 **가로/세로 십자 라인을 빨간색으로 하이라이트**하고, 교차점에 **반투명 돌** 표시


* **착수 인터랙션 (2단계)**:
* 1단계 (임시 선택): 교차점 터치 시 미리보기 및 하단 컨트롤러가 `[착수 확정]` / `[취소]`로 전환
* 2단계 (착수 확정): `[착수 확정]` 버튼 클릭 시 실제 착수로 전환하고 원격 SSH로 GTP 명령 전송


* **AI 분석 시각화**:
* `kata-analyze` streaming data를 수신하여 실시간 winrate graph 및 형세 판단 Bar 즉시 갱신


* **SSH 설정 화면**:
* 외부 파일 탐색기 대신, **SSH Private Key 텍스트 복사/붙여넣기** `<textarea>` 제공
* 서버 프로필(host/port/username/key) **여러 개 등록 및 전환** 가능
* Passphrase가 걸린 Key 감지 시, 1차 버전에서는 처리 대신 **경고 메시지만 표시**


* **게임 모드**:
* (a) 혼자 두며 KataGo 조언을 받는 **연습/분석 모드**
* (c) KataGo가 상대로 직접 착수하는 **대국 모드**
* 두 모드 모두 지원, 기존 **SGF 파일 import**로 이어서 분석/대국 가능


* **다국어(i18n)**:
* 1차 지원 언어: 영어, 한국어 (이후 확장 가능한 구조로 설계)


**나. 백엔드 (Tauri v2 / Rust Core)**

* **SSH 세션 및 Key 보안 저장**:
* 전달받은 SSH Key 텍스트를 Memory 상에서 직접 인증(`userauth_pubkey_memory`)
* 인증 성공 시 Android 외부 권한 요청이 필요 없는 **앱 전용 sandbox 공간**(Android app-specific storage)에 안전하게 보관 및 자동 로드
* 서버 프로필 여러 개를 sandbox 내에 각각 저장, 앱에서 전환 가능


* **GTP I/O 파이프라인**:
* 원격 Server 접속 후 `katago gtp` process 실행
* Channel의 Write 스트림으로 GTP 명령어 전송
* Read Stream을 line 단위로 parsing하여 Tauri Event(`emit`)를 통해 frontend로 실시간 streaming 전달
* GTP 프로세스 무응답 시 **10초 간격으로 재연결 재시도**
* KataGo는 원격 서버에 **사전 설치되어 있다고 가정** (설치/버전 관리는 앱 범위 밖)


---

### **3. 주요 개발 단계별 로드맵**

* **Phase 1: 프론트엔드 바둑판 & 모바일 UI 구축**
* Svelte 5 + Canvas 기반 19x19 바둑판 렌더링
* 오착 방지용 **[임시 선택 (빨간 십자선 + 반투명 돌) $\rightarrow$ 착수 확정]** 인터랙션 완성
* Mobile 세로/가로 반응형 레이아웃 및 컨트롤러 구축


* **Phase 2: Rust 백엔드 SSH & GTP 통신 연동**
* Rust 단에서 SSH 접속 및 Key 텍스트 메모리 인증 구현
* 앱 내부 샌드박스 저장소 연결
* `katago gtp` 프로세스 입출력 파이프라인 수립 및 파싱


* **Phase 3: KataGo 실시간 분석(kata-analyze) 시각화**
* `kata-analyze` 출력을 파싱하여 Svelte 반응형 상태(`$state`)로 변환
* 바둑판 위 추천 수(PV) 및 승률/집수 차이 오버레이 렌더링
* SGF 파일 import 기능 (기존 기보 불러와서 이어서 분석/대국)
* 바둑 규칙(덤, 기본 종국 처리 등) 최소 범위 지원, 이후 확장(중국식/일본식, 핸디캡 등) 가능하도록 규칙 로직을 교체 가능한 모듈로 분리


* **Phase 4: 안드로이드 패키징 및 최적화**
* Tauri v2 Android build 환경 세팅
* Wi-Fi ↔ 모바일 데이터 전환 시에도 유저 입장에서는 세션이 끊기지 않은 것처럼 보이도록 백그라운드 네트워크 세션 유지 및 SSH 재연결(Re-connect) 메커니즘 적용 (세부 구현 방식은 진행하며 논의)
* APK 빌드 및 모바일 실기기 테스트, **APK 직접 배포** (Play Store 미사용)




* **additional: 실시간 집 영역 시각화**
* 현재 바둑판 상태를 실시간으로 읽고 누구의 영역인지 시각화
* if change go board status -> instant change territory visualization


* **additional: Tauri Desktop Build (Linux / Windows)**
* Tauri v2 desktop target 빌드 환경 세팅 (Linux, Windows)
* 데스크탑 환경(마우스/키보드)에 맞춘 착수 인터랙션 및 레이아웃 보정
* Android와 공통 코드베이스 유지, 플랫폼별 조건부 UI/설정 분리
* Windows는 `.exe`(NSIS), Linux는 `.AppImage`로 배포
* Windows는 설치형(NSIS) 외에 **portable(무설치) 실행 파일**도 추가 배포 산출물로 고려 — raw 바이너리를 zip으로 묶는 방식, WebView2 런타임 의존성 처리 방식 결정 필요
* 코드 서명, `tauri-plugin-updater` 자동 업데이트 도입 여부는 미정 — 착수 시점에 결정

---

### **4. 확정된 설계 결정 사항 (2026-09-01)**

* **게임 모드**: (a) 연습/분석 모드, (c) KataGo 대국 모드 둘 다 지원 + SGF import
* **SSH 라이브러리**: **russh** 채택 — pure Rust 구현으로 libssh2(C, 과거 buffer/integer overflow류 CVE 다수) 대비 FFI 공격 표면이 없어 보안성이 높고, Tauri의 async 이벤트 모델과도 자연스럽게 통합됨
* **Key 저장 위치**: Android 앱 전용 sandbox 공간 (앱 삭제 시 함께 삭제, 타 앱 접근 불가)
* **서버 프로필**: 여러 개 등록 및 전환 가능
* **Passphrase 있는 Key**: 1차 버전은 미지원, 경고만 표시 (실제 처리 여부는 추후 논의)
* **KataGo 설치**: 원격 서버에 사전 설치되어 있다고 가정
* **분석 강도 설정 UI**: 로드맵에는 있으나 초기 버전 범위 아님 (추후 Phase로 분리)
* **GTP 무응답 처리**: 10초 간격 재연결 재시도
* **네트워크 전환(Wi-Fi ↔ 모바일 데이터)**: 유저 입장에서 끊김 없이 보이는 것이 목표, 세부 구현은 Phase 2/4 진행하며 결정
* **바둑 규칙**: 초기엔 최소 범위만, 이후 확장 가능하도록 교체형 모듈로 설계
* **다국어**: 영어, 한국어만 지원
* **Android 배포**: APK 직접 배포 (Play Store 미사용)
* **Desktop(Linux/Windows) 배포**: Windows는 `.exe`(NSIS), Linux는 `.AppImage`로 배포. Windows는 **portable(무설치) 버전**도 추가 검토 중 — 설정/SSH Key 저장 위치(exe 옆 폴더 vs `%APPDATA%`)는 **결정 보류**, Desktop 과제 착수 시 재논의. 자동 업데이트는 `tauri-plugin-updater` 사용 가능. 코드 서명·자동 업데이트 도입 여부는 미정
* **macOS**: 당분간 보류

---

### **5. 프로젝트 디렉토리 구조**

Vite + Svelte 5(SvelteKit 미사용, 화면 수가 적어 상태 기반 화면 전환) + Tauri v2 구조로 스캐폴딩 완료.

```
go_webui/
├── PLAN.md
├── README.md
├── package.json / vite.config.ts / tsconfig.json / index.html
├── src/                              # Svelte 5 프론트엔드
│   ├── main.ts, App.svelte, app.css
│   └── lib/
│       ├── screens/                  # GameScreen, SettingsScreen (라우터 없이 상태 전환)
│       ├── components/
│       │   ├── board/                # GoBoard, BoardCanvas, AnalysisOverlay, TerritoryOverlay
│       │   ├── game/                 # GameControls, WinrateGraph
│       │   └── settings/             # ServerProfileList/Form, SshKeyInput
│       ├── stores/                   # board / analysis / connection / serverProfiles (.svelte.ts, $state)
│       ├── i18n/                     # en.json, ko.json, index.ts
│       ├── types/                    # board, gtp, sgf, serverProfile
│       └── utils/                    # sgf.ts (SGF import), goRules.ts (교체형 규칙 모듈)
├── static/                           # 정적 에셋
└── src-tauri/                        # Rust 백엔드
    ├── Cargo.toml (russh, tokio 등), tauri.conf.json, capabilities/default.json
    └── src/
        ├── main.rs, lib.rs, state.rs
        ├── commands/                 # ssh.rs, gtp.rs, profile.rs (invoke 커맨드)
        ├── ssh/                      # client.rs(russh 세션), keystore.rs(sandbox 저장)
        ├── gtp/                      # process.rs(GTP 세션), parser.rs(kata-analyze 파싱)
        └── models/                   # server_profile.rs, game_state.rs
```

- 각 파일은 현재 시그니처/구조만 잡힌 skeleton(TODO 주석)이며 실제 로직은 Phase 1부터 채워나감.
- `src-tauri/tauri.conf.json`의 `bundle.targets`는 `["appimage", "nsis"]`로 4번 결정사항(Windows `.exe`/Linux `.AppImage`) 반영. Android/추가 desktop target은 해당 과제 착수 시 `tauri android init` 등으로 확장.
- Node/npm/Rust toolchain 설치 후 검증 완료: `npm install`, `npm run build`(vite), `svelte-check`, `cargo check`, `cargo fmt` 모두 정상 통과 (2026-09-01).
- Linux 데스크탑 빌드에 필요한 시스템 라이브러리(`webkit2gtk-4.1`, `base-devel`, `appmenu-gtk-module`, `libappindicator-gtk3`, `librsvg` 등)를 pacman으로 설치해야 `cargo check`/`tauri dev`가 동작함 (Tauri v2 공식 Arch Linux 요구사항).
- 아이콘: 현재 `src-tauri/icons/`에는 임시 placeholder PNG(32x32, 128x128, 128x128@2x, icon.png)만 있음. `icon.icns`(macOS)/`icon.ico`(Windows)는 실제 디자인 확정 후 추가 필요 — Windows/macOS 데스크탑 빌드 착수 전 선행 작업.
- **Phase 2(SSH를 통한 원격 KataGo 엔진 연동) 구현 완료 (2026-09-01)**: 서버 프로필 CRUD(sandbox `profiles.json`), russh 기반 SSH 접속(in-memory key 인증, passphrase는 감지만 하고 1차 미지원), `katago gtp` 프로세스 exec, GTP 명령 큐/멀티라인 응답 프레이밍, kata-analyze 스트리밍 라인 파싱+이벤트 emit(오버레이 렌더링은 Phase 3), 무응답 시 10초 간격 재연결(`Notify` 기반 즉시 취소 가능) 전부 구현. Settings 화면에 프로필 등록/연결 UI + 임의 GTP 명령을 보내볼 수 있는 GTP 콘솔 추가. `cargo check`/`cargo fmt`/`cargo test`(parser 유닛 테스트)/`svelte-check`/`npm run build` 전부 통과 확인. **미검증**: 실제 SSH 서버 + katago 대상 end-to-end 연결(사용자가 준비하는 실서버 필요) — host key 검증(TOFU/known_hosts)은 아직 없어 모든 서버 키를 무조건 수락함(`ssh/client.rs`의 `check_server_key` TODO).
