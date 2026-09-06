use serde::{Serialize, Serializer};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[cfg(mobile)]
    #[error(transparent)]
    PluginInvoke(#[from] tauri::plugin::mobile::PluginInvokeError),
    /// 이 플러그인은 Android 온디바이스 엔진 전용이라 다른 플랫폼(desktop/iOS)에서는
    /// 항상 이 에러를 반환한다 - 실제 기능이 없는 게 아니라 애초에 해당 플랫폼에서
    /// 지원 대상이 아님을 뜻한다(desktop.rs 참고).
    #[error("이 플랫폼에서는 로컬 KataGo 엔진을 지원하지 않습니다")]
    UnsupportedPlatform,
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
