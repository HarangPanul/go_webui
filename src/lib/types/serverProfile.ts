export interface ServerProfile {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  // Key 원문은 프론트에 보관하지 않음 (Rust sandbox storage에서 관리)
  hasPassphrase: boolean;
}
