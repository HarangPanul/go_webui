// 데스크탑(Linux/macOS/Windows) 전용: OS의 표준 SSH key 위치(~/.ssh)를 스캔해서
// 서버 프로필 폼에서 바로 골라 채워 넣을 수 있게 해 주는 기능.
//
// Android는 "시스템 SSH key"라는 개념 자체가 없고(앱 샌드박스 밖의 임의 경로를
// 읽을 권한도 없음) - 이 모듈은 desktop cfg에서만 실제로 스캔/읽기를 수행하고,
// mobile cfg에서는 항상 에러를 반환해 프론트가 "이 플랫폼에서는 지원하지 않음"으로
// 처리할 수 있게 한다. lib.rs의 invoke_handler 목록을 플랫폼별로 분기하지 않아도
// 되도록, 커맨드 등록 자체는 항상 하고 이 모듈 내부에서만 cfg로 분기함.
use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::error::AppError;
use crate::ssh::keystore::detect_passphrase;

/// 프론트로 내려가는 감지된 key 요약 정보. key 원문은 포함하지 않고, 사용자가
/// 하나를 선택하면 그때 `load`로 원문을 따로 가져온다.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalSshKeyInfo {
    pub name: String,
    pub path: String,
    pub has_passphrase: bool,
}

#[cfg(desktop)]
fn ssh_dir() -> Result<PathBuf, AppError> {
    let home = dirs::home_dir().ok_or_else(|| AppError::Io("홈 디렉터리를 찾을 수 없습니다".to_string()))?;
    Ok(home.join(".ssh"))
}

/// `~/.ssh` 아래에서 private key로 보이는 파일들을 찾아 반환.
/// - `.pub`, `known_hosts*`, `config`, `authorized_keys`, 숨김 파일은 후보에서 제외.
/// - 남은 파일 중 내용이 "-----BEGIN"으로 시작하는 것만 key로 인정(파일명이
///   id_rsa/id_ed25519 등 관례를 따르지 않는 커스텀 이름의 key도 잡아낼 수 있음).
/// - `~/.ssh`가 아예 없으면 에러가 아니라 빈 목록으로 처리.
#[cfg(desktop)]
pub fn list() -> Result<Vec<LocalSshKeyInfo>, AppError> {
    let dir = ssh_dir()?;
    if !dir.is_dir() {
        return Ok(Vec::new());
    }

    let entries = std::fs::read_dir(&dir)
        .map_err(|e| AppError::Io(format!("{} 읽기 실패: {e}", dir.display())))?;

    let mut keys = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_file() {
            continue;
        }

        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.starts_with('.')
            || name.ends_with(".pub")
            || name.starts_with("known_hosts")
            || name == "config"
            || name == "authorized_keys"
        {
            continue;
        }

        let Ok(content) = std::fs::read_to_string(&path) else {
            continue; // 바이너리거나 읽기 실패 - key 텍스트 파일이 아닐 가능성이 높으므로 건너뜀
        };
        if !content.trim_start().starts_with("-----BEGIN") {
            continue;
        }

        keys.push(LocalSshKeyInfo {
            name: name.to_string(),
            path: path.to_string_lossy().into_owned(),
            has_passphrase: detect_passphrase(&content),
        });
    }

    keys.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(keys)
}

#[cfg(mobile)]
pub fn list() -> Result<Vec<LocalSshKeyInfo>, AppError> {
    Err(AppError::Other(
        "이 플랫폼에서는 로컬 SSH key 자동 감지를 지원하지 않습니다".to_string(),
    ))
}

/// `list()`가 돌려준 경로 중 하나를 골라 실제 key 원문을 읽어 온다.
/// `~/.ssh` 바깥 경로가 넘어오면 거부(방어적 - IPC로 임의 경로를 읽어가는 것을 방지).
#[cfg(desktop)]
pub fn load(path: &str) -> Result<String, AppError> {
    let dir = ssh_dir()?;
    let requested = Path::new(path);

    let canonical_dir = std::fs::canonicalize(&dir)
        .map_err(|e| AppError::Io(format!("{} 접근 실패: {e}", dir.display())))?;
    let canonical_requested = std::fs::canonicalize(requested)
        .map_err(|e| AppError::Io(format!("{path} 접근 실패: {e}")))?;

    if !canonical_requested.starts_with(&canonical_dir) {
        return Err(AppError::InvalidInput(
            "~/.ssh 밖의 경로는 읽을 수 없습니다".to_string(),
        ));
    }

    std::fs::read_to_string(&canonical_requested)
        .map_err(|e| AppError::Io(format!("{path} 읽기 실패: {e}")))
}

#[cfg(mobile)]
pub fn load(_path: &str) -> Result<String, AppError> {
    Err(AppError::Other(
        "이 플랫폼에서는 로컬 SSH key 자동 감지를 지원하지 않습니다".to_string(),
    ))
}
