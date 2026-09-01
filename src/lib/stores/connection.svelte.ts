// SSH/GTP 연결 상태 store: connected / connecting / reconnecting / error
// GTP 무응답 시 10초 간격 재연결 로직의 UI 반영 지점

export type ConnectionStatus =
  | "disconnected"
  | "connecting"
  | "connected"
  | "reconnecting"
  | "error";

function createConnectionStore() {
  let status = $state<ConnectionStatus>("disconnected");
  let lastError = $state<string | null>(null);

  return {
    get status() {
      return status;
    },
    get lastError() {
      return lastError;
    },
    // TODO: invoke("connect_ssh", { profileId }) 및 Tauri event 구독
  };
}

export const connectionStore = createConnectionStore();
