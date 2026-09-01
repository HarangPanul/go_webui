// kata-analyze 스트리밍 결과 store. Tauri event("kata-analyze")를 구독해 갱신

import type { KataAnalyzeResult } from "../types/gtp";

function createAnalysisStore() {
  let latest = $state<KataAnalyzeResult | null>(null);
  let winrateHistory = $state<number[]>([]);

  return {
    get latest() {
      return latest;
    },
    get winrateHistory() {
      return winrateHistory;
    },
    // TODO: Tauri listen("kata-analyze", ...)으로 연결
  };
}

export const analysisStore = createAnalysisStore();
