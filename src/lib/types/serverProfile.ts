export interface ServerProfile {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  engineCommand: string;
  // Key 원문은 프론트에 보관하지 않음 (Rust sandbox storage에서 관리)
  hasPassphrase: boolean;
}

// 프로필 등록/수정 폼 입력값. id가 없으면 신규 생성, 있으면 기존 프로필을 덮어씀
export interface NewServerProfile {
  id?: string;
  name: string;
  host: string;
  port: number;
  username: string;
  privateKey: string;
  engineCommand: string;
}

// 데스크탑에서 ~/.ssh를 스캔해 감지한 key 요약 정보 (원문은 없음 - 선택 시
// loadLocalSshKey로 따로 읽어옴). Android 등에서는 list_local_ssh_keys 자체가
// 에러를 반환하므로 이 타입을 쓸 일이 없음.
export interface LocalSshKeyInfo {
  name: string;
  path: string;
  hasPassphrase: boolean;
}
