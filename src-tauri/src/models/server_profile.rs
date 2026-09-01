use serde::{Deserialize, Serialize};

/// 서버 프로필의 내부/저장용 표현. Private key 원문을 포함하며 sandbox storage
/// (keystore.rs)에만 이 형태 그대로 영속화되고, 프론트로는 절대 내려가지 않는다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerProfile {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    /// SSH private key 원문. sandbox storage에만 영속화되고 프론트로는 내려가지 않음
    pub private_key: String,
    pub has_passphrase: bool,
    /// 원격 서버에서 GTP 엔진을 기동할 명령. 기본값은 "katago gtp"이며, 모델/설정
    /// 파일 경로가 필요하면 사용자가 직접 붙여서 커스터마이즈 (예:
    /// "katago gtp -model model.bin.gz -config gtp.cfg")
    pub engine_command: String,
}

fn default_engine_command() -> String {
    "katago gtp".to_string()
}

/// `save_profile` 커맨드 입력값. `id`가 `None`이면 신규 생성(uuid v4 발급),
/// `Some`이면 기존 프로필을 덮어씀(upsert).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveProfileInput {
    pub id: Option<String>,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub private_key: String,
    #[serde(default = "default_engine_command")]
    pub engine_command: String,
}

/// 프론트엔드로 내려가는 안전한 요약 정보. private key 원문은 포함하지 않음
/// (`src/lib/types/serverProfile.ts`의 `ServerProfile`과 필드 대응).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerProfileInfo {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub engine_command: String,
    pub has_passphrase: bool,
}

impl From<&ServerProfile> for ServerProfileInfo {
    fn from(p: &ServerProfile) -> Self {
        ServerProfileInfo {
            id: p.id.clone(),
            name: p.name.clone(),
            host: p.host.clone(),
            port: p.port,
            username: p.username.clone(),
            engine_command: p.engine_command.clone(),
            has_passphrase: p.has_passphrase,
        }
    }
}
