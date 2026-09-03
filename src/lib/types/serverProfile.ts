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
