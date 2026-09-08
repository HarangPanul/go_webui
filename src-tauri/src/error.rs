// 앱 전역 에러 타입. 실패 종류별로 갈라진 enum이라 `match`로 안전하게 분기할 수
// 있다.
//
// 프런트에는 `{ kind, message }` 구조화된 객체로 직렬화된다. message는 각
// variant의 Display(= #[error(...)] 메시지) 문자열이라 화면에 그대로 보여줄
// 한국어 에러 문구고(appError.ts::appErrorMessage 참고), `kind`는 지금 당장 이
// 태그로 분기하는 프런트 코드는 없지만 나중을 위한 태그다.
//
// Serialize/specta::Type을 derive하지 않고 손으로 구현한 이유: variant마다 실려
// 있는 필드가 다르고(유닛 variant는 아예 없음) 실제로 내보내고 싶은 값은 그 필드가
// 아니라 Display가 계산해주는 메시지라서, derive만으로는 "모든 variant가 항상
// {kind, message} 모양"이 되게 만들 수 없다. 대신 그 모양 그대로인 내부 helper
// 구조체(AppErrorPayload)에 Serialize/specta::Type을 derive해두고 AppError는 거기로
// 위임한다.
use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("SSH에 연결되어 있지 않습니다")]
    NotConnected,

    #[error("{0}")]
    SshConnectFailed(String),

    #[error("{0}")]
    SshAuthFailed(String),

    #[error(
        "서버({host})가 제시한 SSH host key가 이전에 저장해둔 지문과 다릅니다. \
         서버를 재설치했거나 IP를 재할당받았다면 설정 화면에서 이 프로필의 host key 신뢰를 \
         초기화한 뒤 다시 연결하세요. 그런 적이 없다면 중간자 공격(MITM)일 수 있으니 연결하지 마세요. \
         (이전 지문: {expected}, 지금 지문: {actual})"
    )]
    HostKeyMismatch {
        host: String,
        expected: String,
        actual: String,
    },

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

impl AppError {
    /// 프런트로 내려가는 `kind` 태그. variant 이름 그대로라 새 variant를 추가해도
    /// 여기 한 줄만 늘리면 된다.
    fn kind(&self) -> &'static str {
        match self {
            AppError::NotConnected => "NotConnected",
            AppError::SshConnectFailed(_) => "SshConnectFailed",
            AppError::SshAuthFailed(_) => "SshAuthFailed",
            AppError::HostKeyMismatch { .. } => "HostKeyMismatch",
            AppError::PassphraseUnsupported => "PassphraseUnsupported",
            AppError::GtpSendFailed(_) => "GtpSendFailed",
            AppError::ProfileNotFound(_) => "ProfileNotFound",
            AppError::InvalidInput(_) => "InvalidInput",
            AppError::Io(_) => "Io",
            AppError::Other(_) => "Other",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        AppErrorPayload {
            kind: self.kind(),
            message: self.to_string(),
        }
        .serialize(serializer)
    }
}

/// AppError가 실제로 직렬화되는 모양(위 Serialize 참고) - specta::Type을 이 구조체에
/// derive해두고 AppError는 여기로 위임(아래 `impl specta::Type for AppError`)해서,
/// bindings.ts에 손으로 쓴 직렬화와 정확히 일치하는 `{ kind: string; message: string }`
/// 타입이 나오게 한다.
#[derive(Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
struct AppErrorPayload {
    kind: &'static str,
    message: String,
}

impl specta::Type for AppError {
    fn definition(types: &mut specta::Types) -> specta::datatype::DataType {
        AppErrorPayload::definition(types)
    }
}
