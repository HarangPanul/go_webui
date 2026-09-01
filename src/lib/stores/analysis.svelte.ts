// kata-analyze 스트리밍 결과 store. Tauri event("kata-analyze")를 구독해 갱신.
// 바둑판 위 오버레이 렌더링(AnalysisOverlay.svelte) 연결은 Phase 3에서 계속하지만,
// 이벤트 수신 자체는 여기서 끝내둔다.

import { listen } from "@tauri-apps/api/event";
import type { KataAnalyzeResult } from "../types/gtp";

function createAnalysisStore() {
  let latest = $state<KataAnalyzeResult | null>(null);
  let winrateHistory = $state<number[]>([]);

  listen<KataAnalyzeResult>("kata-analyze", (event) => {
    latest = event.payload;
    const top = latest.candidates[0];
    if (top) {
      winrateHistory.push(top.winrate);
    }
  });

  return {
    get latest() {
      return latest;
    },
    get winrateHistory() {
      return winrateHistory;
    },
    reset() {
      latest = null;
      winrateHistory = [];
    },
  };
}

export const analysisStore = createAnalysisStore();
