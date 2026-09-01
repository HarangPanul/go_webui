// SSH/GTP 연결 상태 store: connected / connecting / reconnecting / error
// GTP 무응답 시 10초 간격 재연결 로직의 UI 반영 지점

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type ConnectionStatus =
  | "disconnected"
  | "connecting"
  | "connected"
  | "reconnecting"
  | "error";

interface ConnectionStatusPayload {
  status: ConnectionStatus;
  message?: string | null;
}

function createConnectionStore() {
  let status = $state<ConnectionStatus>("disconnected");
  let lastError = $state<string | null>(null);

  listen<ConnectionStatusPayload>("connection-status", (event) => {
    status = event.payload.status;
    lastError = event.payload.status === "error" ? (event.payload.message ?? null) : null;
  });

  return {
    get status() {
      return status;
    },
    get lastError() {
      return lastError;
    },
    async connect(profileId: string) {
      lastError = null;
      try {
        await invoke("connect_ssh", { profileId });
      } catch (e) {
        // 백엔드가 "error" 이벤트도 emit하지만, invoke 자체가 reject되는 경우(프로필
        // 없음 등 연결 시도 이전 단계의 에러)를 위해 여기서도 상태를 반영
        status = "error";
        lastError = String(e);
        throw e;
      }
    },
    async disconnect() {
      await invoke("disconnect_ssh");
    },
  };
}

export const connectionStore = createConnectionStore();
