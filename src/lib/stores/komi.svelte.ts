// 덤(komi) store. 로컬 게임 트리는 집 계산을 하지 않으므로(그건 전부 KataGo에
// 위임) 이 값 자체는 Rust AppState.komi에 보관되는 값을 그대로 비추는 얇은
// 클라이언트일 뿐 - 실제 효력은 백엔드가 연결된 엔진에 `komi <값>`을 보내야만
// 생긴다(commands::gtp::set_komi / connect_ssh, state.rs::DEFAULT_KOMI 참고).
import { invoke } from "@tauri-apps/api/core";

const DEFAULT_KOMI = 6.5;

function createKomiStore() {
  let komi = $state(DEFAULT_KOMI);

  // 앱 시작 시 백엔드에 저장된 현재 값을 한 번 가져와 동기화
  invoke<number>("get_komi")
    .then((k) => (komi = k))
    .catch((e) => console.error("get_komi 초기 동기화 실패:", e));

  return {
    get value() {
      return komi;
    },
    // 저장과 동시에(연결되어 있다면) 즉시 엔진에도 반영됨 - set_komi 참고.
    async set(value: number) {
      komi = value;
      await invoke("set_komi", { komi: value });
    },
  };
}

export const komiStore = createKomiStore();
