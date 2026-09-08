// 흑/백을 각각 어느 연결된 프로필이 자동으로 둘지 정하는 store. 값은 서버 프로필의
// id고, null이면 그 색은 사람이 둠 - Rust state::EngineAssignment와 필드 대응
// (serde camelCase). 흑/백에 같은 프로필을 배정해도 실제 세션은 하나뿐이므로(백엔드가
// 프로필 id로 중복 연결을 막음) KataGo가 자기 자신과 대국하듯 동작한다.
//
// "프로필을 연결한다"(connectionStore)와 "그 프로필이 흑/백 중 뭘 둔다"(이 store)는
// 완전히 분리된 두 단계다 - 연결은 설정 화면에서, 배정은 메인 화면(GameControls)에서
// 한다. 연결이 끊긴 프로필은 배정에서도 자동으로 풀려야 하므로, connectionStore가
// disconnect()할 때 clearIfAssigned()를 호출해 로컬 상태를 즉시 맞춰준다(백엔드도
// disconnect_ssh에서 같은 처리를 하지만 그걸 알려주는 별도 이벤트는 없음).
import { invoke } from "@tauri-apps/api/core";

export interface EngineAssignment {
  black: string | null;
  white: string | null;
}

function createEngineAssignmentStore() {
  let assignment = $state<EngineAssignment>({ black: null, white: null });

  // 앱 시작 시 Rust 쪽 배정 상태를 한 번 가져와 동기화
  invoke<EngineAssignment>("get_engine_assignment")
    .then((a) => (assignment = a))
    .catch((e) => console.error("get_engine_assignment 초기 동기화 실패:", e));

  return {
    // 흑/백 각각에 배정된 프로필 id(없으면 null) - GameControls의 드롭다운 선택값,
    // engine_sync가 실제로 자동 응수를 반영해 돌려준 결과를 그대로 표시하는 데 사용.
    get assignment() {
      return assignment;
    },
    async setAssignment(color: "black" | "white", profileId: string | null) {
      assignment = { ...assignment, [color]: profileId };
      await invoke("set_engine_assignment", { color, profileId });
    },
    clearIfAssigned(profileId: string) {
      if (assignment.black === profileId || assignment.white === profileId) {
        assignment = {
          black: assignment.black === profileId ? null : assignment.black,
          white: assignment.white === profileId ? null : assignment.white,
        };
      }
    },
  };
}

export const engineAssignmentStore = createEngineAssignmentStore();
