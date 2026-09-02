<script lang="ts">
  // 보드 위 방향키/임시 선택-확정을 제외한, 화면 전역에서 항상 동작하는 키보드
  // 단축키(엔진 연결/흑·백 자동 착수 on-off/Analysis/Ownership/설정 열기)를 한
  // 곳에서 처리하는 컴포넌트. App.svelte 최상단에 한 번만 마운트한다.
  //
  // confirmMove/back/removeLastMove/changeColor(방향키 이동은 제외)도 원래
  // BoardCanvas.svelte에 있었지만 여기로 옮겼다 - 새로 추가된 단축키들
  // (engineConnect/engineWhite/... = "e" + 두 번째 글자로 된 두 글자 시퀀스)의
  // 뒷글자가 기존 단축키(changeColor 기본값 "c" 등)와 겹칠 수 있어서, 리스너가
  // BoardCanvas와 여기 두 곳으로 나뉘어 있으면 "ec"를 누를 때 "e"는 여기서
  // 시퀀스 버퍼에 담기지만 뒤이은 "c"는 BoardCanvas 쪽 리스너가 독립적으로
  // changeColor로 오인해 동시에 발동해버리는 문제가 있었다. 모든
  // keybindingsStore 기반 단축키를 이 컴포넌트 하나로 모아 시퀀스 버퍼를
  // 일원화해야 그런 오작동이 없다.
  import { boardStore } from "../../stores/board.svelte";
  import { analysisStore } from "../../stores/analysis.svelte";
  import { connectionStore } from "../../stores/connection.svelte";
  import { serverProfilesStore } from "../../stores/serverProfiles.svelte";
  import { keybindingsStore, ALL_ACTIONS, type KeyAction } from "../../stores/keybindings.svelte";

  let { activeScreen, onOpenSettings, onCloseSettings }: {
    activeScreen: "game" | "settings";
    onOpenSettings: () => void;
    onCloseSettings: () => void;
  } = $props();

  async function toggleEngineConnection() {
    if (
      connectionStore.status === "connected" ||
      connectionStore.status === "connecting" ||
      connectionStore.status === "reconnecting"
    ) {
      await connectionStore.disconnect();
      return;
    }
    const id = serverProfilesStore.activeProfileId ?? serverProfilesStore.profiles[0]?.id;
    if (!id) return;
    try {
      await connectionStore.connect(id);
      await serverProfilesStore.setActive(id);
    } catch {
      // 실패 원인은 connectionStore.lastError로 노출됨(Settings에서 확인 가능)
    }
  }

  function toggleEngineColor(color: "black" | "white") {
    boardStore.setEngineColor(color, !boardStore.engineColors[color]);
  }

  function toggleAnalysis() {
    if (connectionStore.status !== "connected") return;
    analysisStore.toggleAnalysis();
  }

  function toggleOwnership() {
    if (connectionStore.status !== "connected") return;
    analysisStore.toggleOwnership();
  }

  const ACTION_HANDLERS: Record<KeyAction, () => void> = {
    confirmMove: () => boardStore.confirmMove(),
    back: () => boardStore.goBack(),
    goForward: () => boardStore.goForward(),
    removeLastMove: () => boardStore.removeLastMove(),
    changeColor: () => boardStore.toggleTurn(),
    engineConnect: () => toggleEngineConnection(),
    engineWhite: () => toggleEngineColor("white"),
    engineBlack: () => toggleEngineColor("black"),
    analysis: () => toggleAnalysis(),
    ownership: () => toggleOwnership(),
    // 이미 설정 화면이면 다시 열 필요 없음(history에 중복으로 쌓이는 것도 방지)
    openSettings: () => {
      if (activeScreen === "game") onOpenSettings();
    },
  };

  // "e" 한 글자를 누른 뒤 다음 글자를 기다리는 시퀀스 버퍼. vim의 leader key처럼
  // 시간제한 없이 계속 대기한다 - 앞쪽 키를 눌렀다 떼는(keyup) 것만으로는 버퍼가
  // 비워지지 않고(애초에 keyup은 듣지 않음), 두 번째 키가 실제로 입력되거나 Esc로
  // 취소해야만 비워진다.
  let prefixKey: string | null = null;

  function clearPrefix() {
    prefixKey = null;
  }

  // combo(단일 키 또는 "e"+다음 키를 이어붙인 시퀀스 버퍼)와 일치하는 액션이 있으면
  // 실행하고 true를 반환.
  function dispatch(combo: string): boolean {
    for (const action of ALL_ACTIONS) {
      if (keybindingsStore.matches(action, combo)) {
        ACTION_HANDLERS[action]();
        return true;
      }
    }
    return false;
  }

  function handleKeyDown(evt: KeyboardEvent) {
    const key = evt.key;

    // Esc로 설정 화면 닫기: GTP 콘솔/SSH 프로필 입력창 등에 포커스가 있어도 항상
    // 동작해야 하므로, 입력창 포커스를 무시하는 아래 필터보다 먼저 처리한다.
    // KeybindingsForm이 "키 변경" 대기 중일 때는 그쪽 capture 단계 리스너가
    // stopPropagation으로 이 핸들러까지 전달되는 것 자체를 막으므로, 그 경우엔
    // 여기가 아니라 "키 변경 취소"만 일어나고 설정 화면은 닫히지 않는다.
    if (key === "Escape" && activeScreen === "settings") {
      clearPrefix();
      onCloseSettings();
      evt.preventDefault();
      return;
    }

    // 다른 곳(입력창 등)에 포커스가 있거나 조합키가 눌려있으면 무시
    const target = evt.target as HTMLElement | null;
    if (target && ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName)) return;
    if (evt.ctrlKey || evt.altKey || evt.metaKey) return;

    // Esc: 대기 중이던 시퀀스 버퍼를 완전히 비움(게임 화면에서만 - 설정 화면 닫기는
    // 위에서 이미 처리됨). 버퍼가 비어 있을 때는 여기서 할 일이 없으므로 그대로
    // 흘려보내 다른 Esc 동작(예: pendingMove 취소)을 방해하지 않는다.
    if (key === "Escape") {
      if (prefixKey) {
        clearPrefix();
        evt.preventDefault();
      }
      return;
    }

    if (prefixKey) {
      const combo = prefixKey + key;
      clearPrefix();
      if (dispatch(combo)) {
        evt.preventDefault();
        return;
      }
      // 이전 prefix + 이번 키 조합이 어떤 액션과도 안 맞으면, 이번 키는 완전히
      // 새로운 입력으로 취급해서 아래 로직을 이어서 그대로 탄다.
    }

    if (dispatch(key)) {
      evt.preventDefault();
      return;
    }

    // 지금 누른 키로 시작하는 두 글자 시퀀스가 등록되어 있으면, 시간제한 없이
    // 다음 키를 기다리며 버퍼에 담아둔다(Esc로 취소하거나 다음 키가 올 때까지 유지).
    if (keybindingsStore.isPrefixKey(key)) {
      prefixKey = key;
    }
  }

  $effect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      clearPrefix();
    };
  });
</script>
