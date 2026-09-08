use serde::{Deserialize, Serialize};

/// 이 프로필이 원격 SSH 서버의 `katago gtp`를 쓰는지, 이 기기의 온디바이스 엔진
/// (tauri-plugin-katago-local, Android 전용)을 쓰는지. `Local`이면 host/port/username/
/// privateKey/engineCommand는 전부 의미가 없고 keystore::save가 빈 값으로 저장한다 -
/// 실제로 뭘 쓸지는 gtp::transport::GtpTransport 구현체 선택(services::connection_service)
/// 이 이 값만 보고 결정한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum ProfileKind {
    #[default]
    Ssh,
    Local,
}

/// 서버 프로필의 내부/저장용 표현. Private key 원문을 포함하며 sandbox storage
/// (keystore.rs)에만 이 형태 그대로 영속화되고, 프론트로는 절대 내려가지 않는다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub kind: ProfileKind,
    pub host: String,
    pub port: u16,
    pub username: String,
    /// SSH private key 원문. sandbox storage에만 영속화되고 프론트로는 내려가지 않음
    pub private_key: String,
    pub has_passphrase: bool,
    /// 원격 서버에서 GTP 엔진을 기동할 명령. 기본값은 "katago gtp"이며, 모델/설정
    /// 파일 경로가 필요하면 사용자가 직접 붙여서 커스터마이즈 (예:
    /// "katago gtp -model model.bin.gz -config gtp.cfg"). kind가 Local이면 쓰이지 않음.
    pub engine_command: String,
    /// TOFU(Trust On First Use) 방식 SSH host key 검증에 쓰는, 처음 연결했을 때 서버가
    /// 제시한 host key의 SHA256 지문("SHA256:..." 형식). None이면 아직 한 번도
    /// 연결한 적이 없어 다음 연결 시 무조건 신뢰하고 저장한다(ssh::client::ClientHandler
    /// 참고). Some이면 이후 연결마다 서버가 제시하는 지문과 비교해서 다르면 거부한다 -
    /// 값이 바뀌었다는 건 서버가 재설치됐거나 중간자 공격(MITM)일 수 있다는 뜻.
    /// 기존에 저장된 프로필(이 필드가 생기기 전)을 읽을 때는 없는 것으로 취급.
    #[serde(default)]
    pub host_key_fingerprint: Option<String>,
}

fn default_engine_command() -> String {
    "katago gtp".to_string()
}

/// `save_profile` 커맨드 입력값. `id`가 `None`이면 신규 생성(uuid v4 발급),
/// `Some`이면 기존 프로필을 덮어씀(upsert). kind가 `Local`이면 host/port/username/
/// privateKey/engineCommand는 프론트가 뭘 보내든 keystore::save가 무시하고 빈 값으로
/// 저장한다 - 그 필드들은 SSH 전용이라 폼에서도 애초에 감춰짐.
#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SaveProfileInput {
    pub id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub kind: ProfileKind,
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub port: u16,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub private_key: String,
    #[serde(default = "default_engine_command")]
    pub engine_command: String,
}

/// 프론트엔드로 내려가는 안전한 요약 정보. private key 원문은 포함하지 않음
/// (`src/lib/types/serverProfile.ts`의 `ServerProfile`과 필드 대응).
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ServerProfileInfo {
    pub id: String,
    pub name: String,
    pub kind: ProfileKind,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub engine_command: String,
    pub has_passphrase: bool,
    /// 이 프로필로 이전에 한 번 이상 연결해서 host key 지문을 이미 저장해뒀는지 -
    /// true면 설정 화면이 "host key 신뢰 초기화" 버튼을 보여줄 수 있다. 실제 지문
    /// 값 자체는 검증 외 용도로 쓸 일이 없어 프론트로 내려보내지 않는다.
    pub has_trusted_host_key: bool,
}

impl From<&ServerProfile> for ServerProfileInfo {
    fn from(p: &ServerProfile) -> Self {
        ServerProfileInfo {
            id: p.id.clone(),
            name: p.name.clone(),
            kind: p.kind,
            host: p.host.clone(),
            port: p.port,
            username: p.username.clone(),
            engine_command: p.engine_command.clone(),
            has_passphrase: p.has_passphrase,
            has_trusted_host_key: p.host_key_fingerprint.is_some(),
        }
    }
}
