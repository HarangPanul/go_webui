// 엔진 자동 착수 설정(흑/백 각각 독립 on/off) store. 둘 다 켜져 있으면 엔진이
// 자기 자신과 대국하듯 양쪽을 모두 계속 두고, 둘 다 꺼져 있으면 자동 착수 없이
// 사람이 양쪽을 다 둠 - Rust state::EngineColors와 필드 대응(serde camelCase).
import { invoke } from "@tauri-apps/api/core";

export interface EngineColors {
  black: boolean;
  white: boolean;
}

function createEngineColorsStore() {
  let engineColors = $state<EngineColors>({ black: false, white: false });

  // 앱 시작 시 Rust 쪽 엔진 색 설정의 현재 상태를 한 번 가져와 동기화
  invoke<EngineColors>("get_engine_colors").then((c) => (engineColors = c));

  return {
    // 엔진이 흑/백을 각각 자동으로 둘지 여부 - confirmMove() 후 백엔드가 이 값을
    // 보고 필요하면 자동으로 genmove까지 반영해 돌려주므로, 프론트는 이 값을 버튼
    // on/off 표시에만 사용하면 됨.
    get engineColors() {
      return engineColors;
    },
    async setEngineColor(color: "black" | "white", enabled: boolean) {
      engineColors = { ...engineColors, [color]: enabled };
      await invoke("set_engine_color", { color, enabled });
    },
  };
}

export const engineColorsStore = createEngineColorsStore();
