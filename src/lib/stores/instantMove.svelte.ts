// 설정 화면에서 켜고 끄는 "확인 없이 바로 착수" 옵션. 꺼져 있으면(기본값) 기존과
// 동일하게 임시 선택 -> Confirm Move 버튼(또는 키보드 단축키)으로 확정해야 하고,
// 켜져 있으면 보드를 직접 클릭/터치하는 즉시 착수됨(BoardCanvas 참고 - 마우스는
// 누르는 순간, 터치스크린은 손을 떼는 순간). 키보드로 두는 흐름은 이 설정과 무관하게
// 항상 기존처럼 임시 선택 후 별도 확정 키가 필요함(방향키로 위치를 옮기다 실수로
// 확정되는 일이 없도록).
// localStorage에 저장해 앱을 다시 켜도 유지됨.

const STORAGE_KEY = "go-webui.instantMove";

function loadEnabled(): boolean {
  if (typeof localStorage === "undefined") return false;
  return localStorage.getItem(STORAGE_KEY) === "true";
}

function createInstantMoveStore() {
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

export const instantMoveStore = createInstantMoveStore();
