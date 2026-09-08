// 앱 전용 sandbox 저장소 (Android app-specific storage 등)에 서버 프로필과
// SSH Key 텍스트를 저장/로드. 외부 저장소 권한이 필요 없는 경로만 사용.
//
// 저장 위치: app.path().app_data_dir()/profiles.json
// 내용: { profiles: [...], activeProfileId: ... }

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::server_profile::{ProfileKind, SaveProfileInput, ServerProfile};

const PROFILES_FILE: &str = "profiles.json";

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProfileStore {
    #[serde(default)]
    profiles: Vec<ServerProfile>,
    #[serde(default)]
    active_profile_id: Option<String>,
}

fn store_path(app: &AppHandle) -> Result<PathBuf, AppError> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Io(format!("앱 데이터 디렉터리를 찾을 수 없음: {e}")))?;
    fs::create_dir_all(&dir)
        .map_err(|e| AppError::Io(format!("앱 데이터 디렉터리 생성 실패: {e}")))?;
    Ok(dir.join(PROFILES_FILE))
}

fn read_store(app: &AppHandle) -> Result<ProfileStore, AppError> {
    let path = store_path(app)?;
    if !path.exists() {
        return Ok(ProfileStore::default());
    }
    let text = fs::read_to_string(&path)
        .map_err(|e| AppError::Io(format!("프로필 파일 읽기 실패: {e}")))?;
    if text.trim().is_empty() {
        return Ok(ProfileStore::default());
    }
    serde_json::from_str(&text).map_err(|e| AppError::Other(format!("프로필 파일 파싱 실패: {e}")))
}

fn write_store(app: &AppHandle, store: &ProfileStore) -> Result<(), AppError> {
    let path = store_path(app)?;
    let text = serde_json::to_string_pretty(store)
        .map_err(|e| AppError::Other(format!("프로필 직렬화 실패: {e}")))?;
    fs::write(&path, text).map_err(|e| AppError::Io(format!("프로필 파일 쓰기 실패: {e}")))
}

/// 주어진 private key 텍스트가 passphrase(암호화) 없이 바로 디코딩 가능한지 확인.
/// PLAN.md 결정사항: passphrase 걸린 key는 1차 버전에서 미지원, 저장은 허용하되
/// `has_passphrase`만 표시해서 UI가 경고를 보여줄 수 있게 함.
pub fn detect_passphrase(private_key: &str) -> bool {
    matches!(
        russh_keys::decode_secret_key(private_key, None),
        Err(russh_keys::Error::KeyIsEncrypted)
    )
}

pub fn list(app: &AppHandle) -> Result<Vec<ServerProfile>, AppError> {
    Ok(read_store(app)?.profiles)
}

pub fn get(app: &AppHandle, id: &str) -> Result<Option<ServerProfile>, AppError> {
    Ok(read_store(app)?.profiles.into_iter().find(|p| p.id == id))
}

/// id가 없으면 신규 생성(uuid v4), 있으면 기존 항목을 덮어씀(upsert).
///
/// `input.private_key`가 빈 문자열이면 "키는 그대로 두고 나머지만 수정"으로
/// 취급한다 (프론트는 보안상 기존 key 원문을 절대 내려받지 않으므로, 수정 폼에서
/// key를 다시 입력하지 않고 저장하면 빈 문자열이 온다 - 이 경우 기존 key로 덮어써
/// 유실시키면 안 됨). 신규 생성(id 없음)인데 key가 비어 있으면 에러로 거부한다.
///
/// `input.kind`가 `Local`이면 위 SSH 관련 필드는 전부 무시하고 빈 값으로 저장한다 -
/// 그 필드들은 이 기기의 온디바이스 엔진(tauri-plugin-katago-local)에는 의미가 없고,
/// 폼에서도 애초에 감춰지므로 프론트가 뭘 보내든 여기서 확실히 비운다.
pub fn save(app: &AppHandle, input: SaveProfileInput) -> Result<ServerProfile, AppError> {
    let mut store = read_store(app)?;

    let id = input
        .id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let existing = store.profiles.iter().find(|p| p.id == id).cloned();

    let profile = match input.kind {
        ProfileKind::Local => ServerProfile {
            id: id.clone(),
            name: input.name,
            kind: ProfileKind::Local,
            host: String::new(),
            port: 0,
            username: String::new(),
            private_key: String::new(),
            has_passphrase: false,
            engine_command: String::new(),
            host_key_fingerprint: None,
        },
        ProfileKind::Ssh => {
            let (private_key, has_passphrase) = if input.private_key.is_empty() {
                match &existing {
                    Some(existing) => (existing.private_key.clone(), existing.has_passphrase),
                    None => {
                        return Err(AppError::InvalidInput(
                            "SSH private key를 입력하세요".to_string(),
                        ))
                    }
                }
            } else {
                (
                    input.private_key.clone(),
                    detect_passphrase(&input.private_key),
                )
            };

            // host key 지문은 host/port가 그대로인 수정(이름 변경 등)이면 계속 신뢰를
            // 유지하고, host/port가 바뀌면(다른 서버를 가리키게 됨) 새로 TOFU해야
            // 하므로 지운다. 신규 생성이면 당연히 None.
            let host_key_fingerprint = match &existing {
                Some(existing) if existing.host == input.host && existing.port == input.port => {
                    existing.host_key_fingerprint.clone()
                }
                _ => None,
            };

            ServerProfile {
                id: id.clone(),
                name: input.name,
                kind: ProfileKind::Ssh,
                host: input.host,
                port: input.port,
                username: input.username,
                private_key,
                has_passphrase,
                engine_command: input.engine_command,
                host_key_fingerprint,
            }
        }
    };

    match store.profiles.iter_mut().find(|p| p.id == id) {
        Some(existing) => *existing = profile.clone(),
        None => store.profiles.push(profile.clone()),
    }

    write_store(app, &store)?;
    Ok(profile)
}

pub fn delete(app: &AppHandle, id: &str) -> Result<(), AppError> {
    let mut store = read_store(app)?;
    store.profiles.retain(|p| p.id != id);
    if store.active_profile_id.as_deref() == Some(id) {
        store.active_profile_id = None;
    }
    write_store(app, &store)
}

/// TOFU 첫 연결에서 확인한 host key 지문을 프로필에 저장(`ssh/client.rs`가 연결
/// 직후 호출)하거나, `fingerprint`가 `None`이면 저장된 신뢰를 초기화(설정 화면의
/// "host key 신뢰 초기화" 버튼용 - 서버를 재설치해 host key가 정말로 바뀐 경우
/// 다음 연결을 다시 TOFU로 받아들이게 함)한다.
pub fn set_host_key_fingerprint(
    app: &AppHandle,
    id: &str,
    fingerprint: Option<String>,
) -> Result<(), AppError> {
    let mut store = read_store(app)?;
    let profile = store
        .profiles
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or_else(|| AppError::ProfileNotFound(id.to_string()))?;
    profile.host_key_fingerprint = fingerprint;
    write_store(app, &store)
}

pub fn set_active(app: &AppHandle, id: &str) -> Result<(), AppError> {
    let mut store = read_store(app)?;
    if !store.profiles.iter().any(|p| p.id == id) {
        return Err(AppError::ProfileNotFound(id.to_string()));
    }
    store.active_profile_id = Some(id.to_string());
    write_store(app, &store)
}
