// Android 외 플랫폼용 스텁 - 이 플러그인은 Android 온디바이스 엔진 전용이므로 실제
// 구현 대신 항상 UnsupportedPlatform을 반환한다. 그래도 크레이트 자체는 데스크톱
// 타깃에서도 컴파일되어야(cargo build/check가 desktop에서도 통과해야) 메인 앱의
// Cargo.toml이 플랫폼 분기 없이 이 플러그인을 그냥 의존성으로 추가할 수 있다.
use std::sync::Mutex;

use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Runtime};

use crate::models::*;
use crate::Result;

pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> Result<KatagoLocal<R>> {
    Ok(KatagoLocal {
        app: app.clone(),
        max_visits: Mutex::new(crate::DEFAULT_MAX_VISITS),
    })
}

pub struct KatagoLocal<R: Runtime> {
    #[allow(dead_code)]
    app: AppHandle<R>,
    // mobile.rs와 대칭 유지 - commands/local_engine.rs가 플랫폼 무관하게 컴파일되려면
    // 데스크톱 스텁도 같은 필드가 있어야 한다. 실제로 읽힐 일은 없음
    // (supports_local_engine()이 데스크톱에서 false라 프런트가 애초에 이 값을 다루는
    // UI를 보여주지 않음).
    max_visits: Mutex<u32>,
}

impl<R: Runtime> KatagoLocal<R> {
    pub fn ping(&self, _payload: PingRequest) -> Result<PingResponse> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn gtp_line(&self, _payload: GtpLineRequest) -> Result<GtpLineResponse> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn max_visits(&self) -> u32 {
        *self.max_visits.lock().unwrap()
    }

    pub fn set_max_visits(&self, value: u32) {
        *self.max_visits.lock().unwrap() = value;
    }
}
