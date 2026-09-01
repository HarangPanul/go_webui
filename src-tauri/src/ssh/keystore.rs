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

use crate::models::server_profile::{SaveProfileInput, ServerProfile};

const PROFILES_FILE: &str = "profiles.json";

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProfileStore {
    #[serde(default)]
    profiles: Vec<ServerProfile>,
    #[serde(default)]
    active_profile_id: Option<String>,
}

fn store_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("앱 데이터 디렉터리를 찾을 수 없음: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("앱 데이터 디렉터리 생성 실패: {e}"))?;
    Ok(dir.join(PROFILES_FILE))
}

fn read_store(app: &AppHandle) -> Result<ProfileStore, String> {
    let path = store_path(app)?;
    if !path.exists() {
        return Ok(ProfileStore::default());
    }
    let text = fs::read_to_string(&path).map_err(|e| format!("프로필 파일 읽기 실패: {e}"))?;
    if text.trim().is_empty() {
        return Ok(ProfileStore::default());
    }
    serde_json::from_str(&text).map_err(|e| format!("프로필 파일 파싱 실패: {e}"))
}

fn write_store(app: &AppHandle, store: &ProfileStore) -> Result<(), String> {
    let path = store_path(app)?;
    let text =
        serde_json::to_string_pretty(store).map_err(|e| format!("프로필 직렬화 실패: {e}"))?;
    fs::write(&path, text).map_err(|e| format!("프로필 파일 쓰기 실패: {e}"))
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

pub fn list(app: &AppHandle) -> Result<Vec<ServerProfile>, String> {
    Ok(read_store(app)?.profiles)
}

pub fn get(app: &AppHandle, id: &str) -> Result<Option<ServerProfile>, String> {
    Ok(read_store(app)?.profiles.into_iter().find(|p| p.id == id))
}

/// id가 없으면 신규 생성(uuid v4), 있으면 기존 항목을 덮어씀(upsert).
pub fn save(app: &AppHandle, input: SaveProfileInput) -> Result<ServerProfile, String> {
    let mut store = read_store(app)?;
    let has_passphrase = detect_passphrase(&input.private_key);

    let id = input
        .id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let profile = ServerProfile {
        id: id.clone(),
        name: input.name,
        host: input.host,
        port: input.port,
        username: input.username,
        private_key: input.private_key,
        has_passphrase,
        engine_command: input.engine_command,
    };

    match store.profiles.iter_mut().find(|p| p.id == id) {
        Some(existing) => *existing = profile.clone(),
        None => store.profiles.push(profile.clone()),
    }

    write_store(app, &store)?;
    Ok(profile)
}

pub fn delete(app: &AppHandle, id: &str) -> Result<(), String> {
    let mut store = read_store(app)?;
    store.profiles.retain(|p| p.id != id);
    if store.active_profile_id.as_deref() == Some(id) {
        store.active_profile_id = None;
    }
    write_store(app, &store)
}

pub fn set_active(app: &AppHandle, id: &str) -> Result<(), String> {
    let mut store = read_store(app)?;
    if !store.profiles.iter().any(|p| p.id == id) {
        return Err(format!("존재하지 않는 프로필: {id}"));
    }
    store.active_profile_id = Some(id.to_string());
    write_store(app, &store)
}
