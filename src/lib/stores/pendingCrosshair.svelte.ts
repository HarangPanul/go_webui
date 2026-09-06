// 설정 화면에서 켜고 끄는 "임시 선택 위치 십자선 표시" 옵션. 켜져 있으면(기본값)
// 기존과 동일하게 BoardCanvas가 임시 선택 지점을 지나는 빨간 십자선을 그리고, 꺼져
// 있으면 그 십자선 없이 미리보기 돌만 표시한다(BoardCanvas.svelte 참고).
// localStorage에 저장해 앱을 다시 켜도 유지됨.

const STORAGE_KEY = "go-webui.pendingCrosshair";

function loadEnabled(): boolean {
  if (typeof localStorage === "undefined") return true;
  const stored = localStorage.getItem(STORAGE_KEY);
  return stored === null ? true : stored === "true";
}

function createPendingCrosshairStore() {
  let enabled = $state(loadEnabled());

  return {
    get enabled() {
      return enabled;
    },
    set(value: boolean) {
      enabled = value;
      if (typeof localStorage !== "undefined") {
        localStorage.setItem(STORAGE_KEY, String(value));
      }
    },
  };
}

export const pendingCrosshairStore = createPendingCrosshairStore();
