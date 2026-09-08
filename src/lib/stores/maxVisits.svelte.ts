// 로컬 온디바이스 엔진(tauri-plugin-katago-local)의 genmove가 쓸 Mcts 시뮬레이션 수
// store. komi.svelte.ts와 완전히 같은 패턴 - Rust AppState.max_visits를 그대로 비추는
// 얇은 클라이언트일 뿐이고, 실제 효력은 백엔드가 연결된 엔진에 `max_visits <값>`을
// 보내야만 생긴다(commands::gtp::set_max_visits, state.rs::DEFAULT_MAX_VISITS 참고).
// 원격 SSH KataGo에는 의미 없는 값이지만(그쪽은 이 확장 명령을 모름 - 무해하게
// 무시됨), 설정 화면 하나로 두 종류 엔진을 함께 다루는 게 자연스러워 komi와 같은
// 자리에 둔다.
import { invoke } from "@tauri-apps/api/core";

const DEFAULT_MAX_VISITS = 16;

function createMaxVisitsStore() {
  let maxVisits = $state(DEFAULT_MAX_VISITS);

  // 앱 시작 시 백엔드에 저장된 현재 값을 한 번 가져와 동기화
  invoke<number>("get_max_visits")
    .then((v) => (maxVisits = v))
    .catch((e) => console.error("get_max_visits 초기 동기화 실패:", e));

  return {
    get value() {
      return maxVisits;
    },
    // 저장과 동시에(연결되어 있다면) 즉시 엔진에도 반영됨 - set_max_visits 참고.
    async set(value: number) {
      maxVisits = value;
      await invoke("set_max_visits", { maxVisits: value });
    },
  };
}

export const maxVisitsStore = createMaxVisitsStore();
