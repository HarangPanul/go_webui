// 플랫폼별로 UI를 갈라야 할 때(지금은 "Local" 프로필 종류를 보여줄지) 쓰는 조회용
// store. Rust commands::platform::supports_local_engine을 앱 시작 시 한 번만 물어보고
// 캐시한다 - Android가 아니면 항상 false이므로 ServerProfileForm이 SSH/Local 선택
// UI 자체를 감춘다.
import { invoke } from "@tauri-apps/api/core";

function createPlatformStore() {
  let supportsLocalEngine = $state(false);

  invoke<boolean>("supports_local_engine")
    .then((v) => (supportsLocalEngine = v))
    .catch((e) => console.error("supports_local_engine 초기 동기화 실패:", e));

  return {
    get supportsLocalEngine() {
      return supportsLocalEngine;
    },
  };
}

export const platformStore = createPlatformStore();
