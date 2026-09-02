// 앱 전역 에러 타입. 예전에는 모든 커맨드가 `Result<T, String>`을 반환해서 실패
// 종류를 코드로 구분할 방법이 없었는데(문자열 내용을 보고 추측하는 수밖에 없었음),
// 이제는 Rust 쪽에서 종류별로 갈라진 enum이라 앞으로 새 코드를 짤 때 `match`로
// 안전하게 분기할 수 있다.
//
// Serialize는 일부러 derive하지 않고 Display(= 아래 각 variant의 #[error(...)]
// 메시지) 문자열 하나만 직렬화하도록 손으로 구현했다 - 프런트엔드가 지금
// invoke()가 reject한 값을 항상 순수 문자열로 취급하고 있어(`String(e)`로 그대로
// 표시, GtpConsole.svelte/ServerProfileForm.svelte/SshKeyInput.svelte/
// connection.svelte.ts 등) 그 계약을 이번 단계에서는 그대로 유지해야 한다. `kind`
// 태그를 프런트에 구조화된 형태로 노출하는 건 tauri-specta를 실제로 연결하고
// 프런트도 함께 구조화된 에러를 소비하도록 고치는 단계(리팩터 계획의 6단계)에서
// 한 번에 다시 설계한다 - 지금 어중간하게 태그를 노출하면 프런트는 여전히
// 문자열만 기대하는데 값은 객체로 바뀌어(특히 필드가 없는 variant는 메시지조차
// 없는 `{"kind":"NotConnected"}` 형태가 됨) 에러 문구가 "[object Object]"로
// 보이는 회귀가 생긴다.
//
// 각 variant의 메시지(Display)는 기존에 각 실패 지점에서 손으로 만들던 한국어
// 에러 문자열을 그대로 옮긴 것 - 사용자에게 보여줄 문구 자체는 바뀌지 않았고,
// "어떤 종류의 실패인지"만 Rust 타입으로 드러나게 한 것.
use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("SSH에 연결되어 있지 않습니다")]
    NotConnected,

    #[error("{0}")]
    SshConnectFailed(String),

    #[error("{0}")]
    SshAuthFailed(String),

    #[error("passphrase가 걸린 key는 아직 지원하지 않습니다. passphrase 없는 key를 사용해주세요.")]
    PassphraseUnsupported,

    #[error("{0}")]
    GtpSendFailed(String),

    #[error("존재하지 않는 프로필: {0}")]
    ProfileNotFound(String),

    #[error("{0}")]
    InvalidInput(String),

    #[error("{0}")]
    Io(String),

    #[error("{0}")]
    Other(String),
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
