// Android 온디바이스 KataGo(ExecuTorch, android_kata 프로젝트에서 검증됨) 엔진을
// go_webui의 GtpSession에 gtp::transport::GtpTransport 구현체로 연결하기 위한 Tauri
// 모바일 플러그인. 지금은 배선 자체(Cargo 경로 의존성 -> gen/android 자동 Gradle 등록
// -> JNI 왕복)만 ping 하나로 확인하는 단계이고, 실제 GTP 명령 셸(boardsize/play/
// genmove/undo/kata-analyze)은 이 배선이 실기기에서 검증된 뒤 추가한다.
//
// desktop.rs/mobile.rs로 나뉘는 이유: 이 플러그인은 Android 전용이지만, 크레이트
// 자체는 데스크톱 타깃에서도 컴파일되어야(그래야 메인 앱의 Cargo.toml이 플랫폼 분기
// 없이 그냥 의존성으로 추가할 수 있음) 한다 - desktop.rs는 항상 UnsupportedPlatform을
// 반환하는 스텁.
use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

pub use models::*;

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

mod commands;
mod error;
mod models;

pub use error::{Error, Result};

// android/.../GtpShell.kt의 DEFAULT_MAX_VISITS와 맞춰둠 - desktop.rs/mobile.rs 둘 다
// KatagoLocal 초기값으로 씀(commands/local_engine.rs, gtp/android_transport.rs 참고).
pub const DEFAULT_MAX_VISITS: u32 = 16;

#[cfg(desktop)]
use desktop::KatagoLocal;
#[cfg(mobile)]
use mobile::KatagoLocal;

/// `app.katago_local()`로 플러그인 핸들에 접근하기 위한 확장 트레잇.
pub trait KatagoLocalExt<R: Runtime> {
    fn katago_local(&self) -> &KatagoLocal<R>;
}

impl<R: Runtime, T: Manager<R>> crate::KatagoLocalExt<R> for T {
    fn katago_local(&self) -> &KatagoLocal<R> {
        self.state::<KatagoLocal<R>>().inner()
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("katago-local")
        .invoke_handler(tauri::generate_handler![commands::ping, commands::gtp_line])
        .setup(|app, api| {
            #[cfg(mobile)]
            let katago_local = mobile::init(app, api)?;
            #[cfg(desktop)]
            let katago_local = desktop::init(app, api)?;
            app.manage(katago_local);
            Ok(())
        })
        .build()
}
