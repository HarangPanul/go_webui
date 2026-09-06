// 어느 연결된 프로필을 kata-analyze(Analysis/Ownership 표시)에 쓸지 사용자가 직접
// 고르는 store. 값은 서버 프로필의 id, null이면 백엔드(commands::gtp::
// session_for_analysis)가 기존 방식대로 자동으로 고른다(지금 차례 색에 배정된 세션 ->
// 없으면 연결된 아무 세션).
//
// 왜 필요한가: self-play처럼 흑/백 둘 다 같은 서버에 배정해 genmove가 끊임없이 도는
// 상황에서는, 그 서버가 kata-analyze도 함께 맡으면 GTP 프로토콜 특성상(한 세션이
// genmove/kata-analyze를 동시에 못 함 - 다음 입력이 오면 스트리밍이 멈춤) 매 genmove가
// 곧바로 kata-analyze를 인터럽트해버려 분석/ownership이 사실상 전혀 나오지 않는다.
// 다른 서버를 하나 더 연결해 이 store로 지정해두면 genmove와 분석이 서로 다른 세션을
// 써서 서로 끊지 않는다 - self-play 속도를 늦추거나 자동으로 두 번째 연결을 만들지
// 않고, 사용자가 이미 연결해둔 것 중 고르게 하는 가장 단순한 형태.
import { invoke } from "@tauri-apps/api/core";

function createAnalysisEngineStore() {
  let profileId = $state<string | null>(null);

  // 앱 시작 시 Rust 쪽 상태를 한 번 가져와 동기화
  invoke<string | null>("get_analysis_engine").then((id) => (profileId = id));

  return {
    // 지정된 게 없으면 null(자동 선택) - GameControls의 드롭다운 선택값.
    get profileId() {
      return profileId;
    },
    async setProfileId(id: string | null) {
      profileId = id;
      await invoke("set_analysis_engine", { profileId: id });
    },
    // 연결이 끊긴 프로필이 계속 지정된 채로 남지 않게 한다(connectionStore.disconnect
    // 참고, engineAssignmentStore.clearIfAssigned와 같은 패턴). 백엔드도
    // disconnect_ssh에서 같은 처리를 하지만 그걸 알려주는 별도 이벤트가 없다.
    clearIfAssigned(id: string) {
      if (profileId === id) profileId = null;
    },
  };
}

export const analysisEngineStore = createAnalysisEngineStore();
