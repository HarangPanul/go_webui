// "ec" 단축키로 여는 엔진 선택창의 열림/닫힘 + 커서 위치만 다루는 store.
// 실제 연결/해제는 connectionStore에 그대로 위임하고, 여기서는 "지금 몇 번째
// 프로필이 선택되어 있는지"와 "선택창이 떠 있는지"만 상태로 갖는다 - 키 입력
// 자체는 KeyboardShortcuts.svelte가 (다른 모든 단축키와 마찬가지로 한 곳에서)
// 받아 이 store의 메서드를 호출하고, EngineConnectPicker.svelte는 그 상태를
// 그대로 그려서 보여주기만 한다.

import { serverProfilesStore } from "./serverProfiles.svelte";
import { connectionStore } from "./connection.svelte";

function createEngineConnectPickerStore() {
  let isOpen = $state(false);
  let selectedIndex = $state(0);

  // 목록 범위를 벗어나지 않도록 감싸기(위/아래 이동이 맨 끝에서 반대쪽 끝으로
  // 순환하게) - 프로필이 0개면 그냥 0으로 고정.
  function clampIndex() {
    const len = serverProfilesStore.profiles.length;
    selectedIndex = len === 0 ? 0 : ((selectedIndex % len) + len) % len;
  }

  return {
    get isOpen() {
      return isOpen;
    },
    get selectedIndex() {
      return selectedIndex;
    },
    // 등록된 프로필이 하나도 없으면 열어봐야 빈 창만 뜨므로 그냥 무시.
    // 열 때 커서는 "지금 활성 프로필"(가장 최근에 연결/전환한 프로필) 위치에서
    // 시작해, 이미 쓰고 있던 엔진을 다시 찾아 화살표로 여러 번 넘길 필요가 없게 함.
    open() {
      if (serverProfilesStore.profiles.length === 0) return;
      const activeIdx = serverProfilesStore.profiles.findIndex(
        (p) => p.id === serverProfilesStore.activeProfileId,
      );
      selectedIndex = activeIdx >= 0 ? activeIdx : 0;
      isOpen = true;
    },
    close() {
      isOpen = false;
    },
    moveUp() {
      selectedIndex -= 1;
      clampIndex();
    },
    moveDown() {
      selectedIndex += 1;
      clampIndex();
    },
    // index를 생략하면 지금 커서(selectedIndex)가 가리키는 프로필을 대상으로 함 -
    // 화살표+Enter 경로. 마우스로 항목을 직접 클릭했을 때는 커서 위치와 무관하게
    // 그 항목을 바로 대상으로 삼기 위해 index를 명시적으로 넘겨받는다.
    // 이미 연결/연결 중/재연결 중이면 해제하고, 그 외(연결 안 됨/에러)면 새로
    // 연결한 뒤 활성 프로필로 전환한다 - GameControls 등 "활성 프로필" 기준으로
    // 동작하는 다른 단축키(엔진 색 배정 등)와 같은 기준을 공유하기 위함.
    async confirm(index?: number) {
      const target = index ?? selectedIndex;
      const profile = serverProfilesStore.profiles[target];
      isOpen = false;
      if (!profile) return;

      const status = connectionStore.statusFor(profile.id);
      if (status === "connected" || status === "connecting" || status === "reconnecting") {
        await connectionStore.disconnect(profile.id);
        return;
      }
      try {
        await connectionStore.connect(profile.id);
        await serverProfilesStore.setActive(profile.id);
      } catch {
        // 실패 원인은 connectionStore.errorFor(id)로 노출됨(Settings에서 확인 가능)
      }
    },
  };
}

export const engineConnectPickerStore = createEngineConnectPickerStore();
