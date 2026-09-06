// Android 쪽 실제 구현으로 연결. register_android_plugin은 Rust에서 JNI를 통해 직접
// android/src/main/java/com/plugin/katagolocal/KatagoLocalPlugin.kt를 인스턴스화하고
// PluginManager에 등록한다 - MainActivity를 손으로 고쳐 등록할 필요가 없다(클래스가
// 이 크레이트의 Gradle 모듈로 :app에 포함되어 있기만 하면 됨).
use std::sync::Mutex;

use serde::de::DeserializeOwned;
use tauri::{
    plugin::{PluginApi, PluginHandle},
    AppHandle, Runtime,
};

use crate::models::*;
use crate::Result;

const PLUGIN_IDENTIFIER: &str = "com.plugin.katagolocal";

pub fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> Result<KatagoLocal<R>> {
    let handle = api.register_android_plugin(PLUGIN_IDENTIFIER, "KatagoLocalPlugin")?;
    Ok(KatagoLocal {
        handle,
        max_visits: Mutex::new(crate::DEFAULT_MAX_VISITS),
    })
}

pub struct KatagoLocal<R: Runtime> {
    handle: PluginHandle<R>,
    // GtpShell.kt의 DEFAULT_MAX_VISITS와 맞춰둠. 메인 크레이트의
    // AppState(SSH 세션 등 범용 상태)에는 넣지 않고 여기 두는 이유: 이 값은 로컬
    // 온디바이스 엔진에만 의미가 있고, (재)연결마다 새로 만들어지는
    // AndroidLocalTransport 인스턴스보다 오래 살아야 한다(연결이 한 번 끊겼다 다시
    // 붙어도 값을 잃으면 안 됨) - 앱 전체 수명을 갖는 이 플러그인 상태가 자연스러운
    // 자리다. commands/local_engine.rs가 프런트(get/set_max_visits)에 노출하고,
    // gtp::android_transport::AndroidLocalTransport::extra_resync_commands가
    // (재)연결마다 이 값을 읽어 엔진에 보낸다.
    max_visits: Mutex<u32>,
}

impl<R: Runtime> KatagoLocal<R> {
    pub fn ping(&self, payload: PingRequest) -> Result<PingResponse> {
        self.handle
            .run_mobile_plugin("ping", payload)
            .map_err(Into::into)
    }

    pub fn gtp_line(&self, payload: GtpLineRequest) -> Result<GtpLineResponse> {
        self.handle
            .run_mobile_plugin("gtpLine", payload)
            .map_err(Into::into)
    }

    pub fn max_visits(&self) -> u32 {
        *self.max_visits.lock().unwrap()
    }

    pub fn set_max_visits(&self, value: u32) {
        *self.max_visits.lock().unwrap() = value;
    }
}
