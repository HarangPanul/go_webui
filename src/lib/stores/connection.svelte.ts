// SSH/GTP 연결 상태 store: connected / connecting / reconnecting / error
// GTP 무응답 시 10초 간격 재연결 로직의 UI 반영 지점

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { appErrorMessage } from "../appError";

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
    // reconnecting 상태도 메시지가 오면 보여줘야 함 - 그래야 재연결이 왜 계속
    // 실패하는지(잘못된 engine 명령 등) 화면에서 바로 알 수 있다. connected/
    // disconnected로 정상 전이하면 지난 에러는 지운다.
    if (event.payload.message) {
      lastError = event.payload.message;
    } else if (status === "connected" || status === "disconnected") {
      lastError = null;
    }
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
        lastError = appErrorMessage(e);
        throw e;
      }
    },
    async disconnect() {
      await invoke("disconnect_ssh");
    },
  };
}

export const connectionStore = createConnectionStore();
