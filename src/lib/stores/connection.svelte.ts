// SSH/GTP 연결 상태 store: 프로필 id별 disconnected/connecting/connected/
// reconnecting/error 상태를 각각 추적한다 - 여러 프로필을 동시에 연결해둘 수 있으므로
// (여러 서버 동시 연결) 더 이상 앱 전체에 상태가 하나뿐이지 않다.
// GTP 무응답 시 10초 간격 재연결 로직의 UI 반영 지점이자, 그 재연결이 성공했을 때
// 흑/백 자동 응수(엔진끼리 대국)를 다시 깨우는 지점이기도 함(아래 리스너 참고).

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { appErrorMessage } from "../appError";
import { engineAssignmentStore } from "./engineAssignment.svelte";

export type ConnectionStatus =
  | "disconnected"
  | "connecting"
  | "connected"
  | "reconnecting"
  | "error";

interface ConnectionStatusPayload {
  profileId: string;
  status: ConnectionStatus;
  message?: string | null;
}

// TopBar처럼 프로필별 상태를 다 보여줄 공간이 없는 곳에서 "지금 전체적으로 신경 써야
// 할 상태가 뭔지"를 한눈에 보여주기 위한 우선순위(앞쪽일수록 더 눈에 띄어야 함).
const STATUS_PRIORITY: ConnectionStatus[] = [
  "error",
  "reconnecting",
  "connecting",
  "connected",
  "disconnected",
];

function createConnectionStore() {
  let statuses = $state<Record<string, ConnectionStatus>>({});
  let errors = $state<Record<string, string | null>>({});

  listen<ConnectionStatusPayload>("connection-status", (event) => {
    const { profileId, status, message } = event.payload;
    const prevStatus = statuses[profileId];
    statuses = { ...statuses, [profileId]: status };
    // reconnecting 상태도 메시지가 오면 보여줘야 함 - 그래야 재연결이 왜 계속
    // 실패하는지(잘못된 engine 명령 등) 화면에서 바로 알 수 있다. connected/
    // disconnected로 정상 전이하면 지난 에러는 지운다.
    if (message) {
      errors = { ...errors, [profileId]: message };
    } else if (status === "connected" || status === "disconnected") {
      errors = { ...errors, [profileId]: null };
    }

    // 끊겼다(reconnecting/error) 다시 붙은 프로필이 흑/백 중 한쪽에 배정되어 있으면
    // 자동 응수 루프(request_engine_move_if_needed)를 재배정으로 다시 깨운다 - 그
    // 루프는 세션이 끊기는 순간(GtpSession::send 실패) 조용히 멈춰버리고, SSH 연결
    // 자체가 자동으로 되살아나더라도 누가 다시 배정해주지 않으면 스스로 이어지지
    // 않는다(set_engine_assignment가 매번 배정 직후 이어서 자동 응수를 시도하는 게
    // 유일한 시작점 - commands/gtp.rs 참고). 같은 프로필이 흑/백 둘 다일 수도 있는데,
    // 그 경우 세션이 하나뿐이라 한쪽만 재배정해도 그 루프 하나가 두 색을 이어서
    // 처리한다 - 둘 다 재배정하면 같은 세션에 genmove가 동시에 두 번 나가는 경합이
    // 생길 수 있어 반드시 한쪽만 골라야 한다.
    if (status === "connected" && prevStatus && prevStatus !== "connected") {
      const { assignment } = engineAssignmentStore;
      const color =
        assignment.black === profileId ? "black" : assignment.white === profileId ? "white" : null;
      if (color) {
        invoke("set_engine_assignment", { color, profileId }).catch(() => {});
      }
    }
  });

  return {
    statusFor(profileId: string): ConnectionStatus {
      return statuses[profileId] ?? "disconnected";
    },
    errorFor(profileId: string): string | null {
      return errors[profileId] ?? null;
    },
    // 지금 연결된(status === "connected") 프로필 id 목록 - GameControls의 흑/백 배정
    // 드롭다운 후보로 사용(연결 안 된 프로필은 배정해봐야 실제로 둘 세션이 없음).
    get connectedProfileIds() {
      return Object.keys(statuses).filter((id) => statuses[id] === "connected");
    },
    // 여러 프로필 상태를 하나로 요약(TopBar 등 공간이 좁은 곳 전용) - 위 우선순위대로
    // "지금 가장 신경 써야 할 상태"를 고른다. 아무 프로필도 건드린 적 없으면 disconnected.
    get overallStatus(): ConnectionStatus {
      const values = Object.values(statuses);
      for (const s of STATUS_PRIORITY) {
        if (values.includes(s)) return s;
      }
      return "disconnected";
    },
    async connect(profileId: string) {
      errors = { ...errors, [profileId]: null };
      try {
        await invoke("connect_ssh", { profileId });
      } catch (e) {
        // 백엔드가 "error" 이벤트도 emit하지만, invoke 자체가 reject되는 경우(프로필
        // 없음 등 연결 시도 이전 단계의 에러)를 위해 여기서도 상태를 반영
        statuses = { ...statuses, [profileId]: "error" };
        errors = { ...errors, [profileId]: appErrorMessage(e) };
        throw e;
      }
    },
    async disconnect(profileId: string) {
      await invoke("disconnect_ssh", { profileId });
      // 백엔드도 disconnect_ssh에서 이 프로필의 흑/백 배정을 풀지만, 그걸 알려주는
      // 별도 이벤트가 없으므로 프런트 상태도 여기서 바로 맞춰준다.
      engineAssignmentStore.clearIfAssigned(profileId);
    },
  };
}

export const connectionStore = createConnectionStore();
