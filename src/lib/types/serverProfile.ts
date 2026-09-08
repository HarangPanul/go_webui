// "ssh": 원격 서버의 `katago gtp`를 SSH로 씀 (host/port/username/privateKey/
// engineCommand 사용). "local": 이 기기의 온디바이스 엔진(Android 전용,
// tauri-plugin-katago-local)을 씀 - 그 경우 나머지 필드는 전부 무시됨.
export type ProfileKind = "ssh" | "local";

export interface ServerProfile {
  id: string;
  name: string;
  kind: ProfileKind;
  host: string;
  port: number;
  username: string;
  engineCommand: string;
  // Key 원문은 프론트에 보관하지 않음 (Rust sandbox storage에서 관리)
  hasPassphrase: boolean;
  // TOFU 방식 SSH host key 검증에서 지문을 이미 한 번 신뢰해 저장해뒀는지. true일 때만
  // "host key 신뢰 초기화" 버튼을 보여줄 수 있다(ssh::client::ClientHandler 참고).
  // kind가 "local"이면 항상 false.
  hasTrustedHostKey: boolean;
}

// 프로필 등록/수정 폼 입력값. id가 없으면 신규 생성, 있으면 기존 프로필을 덮어씀.
// kind가 "local"이면 host/port/username/privateKey/engineCommand는 백엔드가
// 무시하고 빈 값으로 저장하므로(ssh::keystore::save 참고) 아무 값이나 넘겨도 무방.
export interface NewServerProfile {
  id?: string;
  name: string;
  kind: ProfileKind;
  host: string;
  port: number;
  username: string;
  privateKey: string;
  engineCommand: string;
}
