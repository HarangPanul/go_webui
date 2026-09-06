// 엔진이 기권(resign)했을 때 그 결과("B+R"/"W+R")를 잠깐 들고 있는 store.
// engine_sync.rs::request_engine_move_if_needed가 genmove 응답이 "resign"이면
// 기권한 색을 payload로 실어 "engine-resigned"를 emit하고, 여기서 그걸 받아
// resignedColor에 저장한다 - WinrateGraph가 이 값이 있으면 scoreLead 숫자 대신
// "B+R"/"W+R"을 보여준다.
//
// 이 값은 "지금 보드가 그 기권 시점 그대로"일 때만 유효하므로, 사람이 이어서
// 두거나 뒤로/앞으로 이동하는 등 보드 상태가 조금이라도 바뀌면 gameTree.svelte.ts의
// 각 커맨드 래퍼가 clear()를 호출해 지운다.
import { listen } from "@tauri-apps/api/event";
import type { Color } from "../generated/bindings";

function createGameResultStore() {
  let resignedColor = $state<Color | null>(null);

  listen<Color>("engine-resigned", (event) => {
    resignedColor = event.payload;
  });

  return {
    get resignedColor() {
      return resignedColor;
    },
    clear() {
      resignedColor = null;
    },
  };
}

export const gameResultStore = createGameResultStore();
