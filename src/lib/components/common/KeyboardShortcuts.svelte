<script lang="ts">
  // 보드 위 방향키/임시 선택-확정을 제외한, 화면 전역에서 항상 동작하는 키보드
  // 단축키(엔진 연결/흑·백 자동 착수 on-off/Analysis/Ownership/설정 열기)를 한
  // 곳에서 처리하는 컴포넌트. App.svelte 최상단에 한 번만 마운트한다.
  //
  // confirmMove/back/removeLastMove/changeColor(방향키 이동은 제외)도 반드시 이
  // 컴포넌트 하나에서만 처리해야 한다 - 두 글자 시퀀스 단축키(engineConnect/
  // engineWhite/... = "e" + 두 번째 글자)의 뒷글자가 기존 단축키(changeColor
  // 기본값 "c" 등)와 겹칠 수 있어서, 리스너가 여러 컴포넌트로 나뉘어 있으면 "ec"를
  // 누를 때 "e"는 여기서 시퀀스 버퍼에 담기지만 뒤이은 "c"를 다른 리스너가
  // 독립적으로 changeColor로 오인해 동시에 발동해버리는 오작동이 생긴다. 모든
  // keybindingsStore 기반 단축키를 이 컴포넌트 하나로 모아 시퀀스 버퍼를
  // 일원화해야 그런 문제가 없다.
  import { gameTreeStore } from "../../stores/gameTree.svelte";
  import { pendingMoveStore } from "../../stores/pendingMove.svelte";
  import { engineAssignmentStore } from "../../stores/engineAssignment.svelte";
  import { analysisStore } from "../../stores/analysis.svelte";
  import { connectionStore } from "../../stores/connection.svelte";
  import { serverProfilesStore } from "../../stores/serverProfiles.svelte";
  import { engineConnectPickerStore } from "../../stores/engineConnectPicker.svelte";
  import { keybindingsStore, ALL_ACTIONS, type KeyAction } from "../../stores/keybindings.svelte";

  let { activeScreen, onOpenSettings, onCloseSettings }: {
    activeScreen: "game" | "settings";
    onOpenSettings: () => void;
    onCloseSettings: () => void;
  } = $props();

  // 이미 그 색에 배정되어 있으면 해제(사람이 둠으로), 아니면 "가장 최근에 활성화한
  // 프로필"을 배정한다 - toggleEngineConnection과 같은 기준으로 대상을 고른다.
  // 그 프로필이 지금 연결되어 있지 않으면(예: 아직 연결 전) 아무 일도 하지 않는다.
  function toggleEngineColor(color: "black" | "white") {
    if (engineAssignmentStore.assignment[color]) {
      engineAssignmentStore.setAssignment(color, null);
      return;
    }
    const id = serverProfilesStore.activeProfileId ?? serverProfilesStore.profiles[0]?.id;
    if (!id || connectionStore.statusFor(id) !== "connected") return;
    engineAssignmentStore.setAssignment(color, id);
  }

  function toggleAnalysis() {
    if (connectionStore.connectedProfileIds.length === 0) return;
    analysisStore.toggleAnalysis();
  }

  function toggleOwnership() {
    if (connectionStore.connectedProfileIds.length === 0) return;
    analysisStore.toggleOwnership();
  }

  const ACTION_HANDLERS: Record<KeyAction, () => void> = {
    confirmMove: () => gameTreeStore.confirmMove(),
    back: () => gameTreeStore.goBack(),
    goForward: () => gameTreeStore.goForward(),
    removeLastMove: () => gameTreeStore.removeLastMove(),
    cancelPendingMove: () => pendingMoveStore.cancelPending(),
    changeColor: () => gameTreeStore.toggleTurn(),
    // 바로 연결하지 않고 선택창을 띄운다 - 실제 연결 대상 선택은 그 창이 뜬 동안
    // 아래 handleKeyDown의 위/아래 화살표·Enter 처리가 담당(engineConnectPickerStore 참고).
    engineConnect: () => engineConnectPickerStore.open(),
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

    // 엔진 선택창이 떠 있는 동안은 그 어떤 단축키(설정 화면 Esc 닫기 포함)보다
    // 이 처리를 최우선으로 가로챈다 - 위/아래로 항목을 고르고 Enter로 확정,
    // Esc로 취소하는 것 외의 키는(다른 단축키와 겹쳐 오작동하지 않도록) 전부 무시.
    // BoardCanvas.svelte의 방향키(임시 선택 이동) 리스너도 같은 store를 확인해
    // 이 창이 떠 있을 땐 스스로 멈춘다(그쪽 주석 참고).
    if (engineConnectPickerStore.isOpen) {
      evt.preventDefault();
      switch (key) {
        case "ArrowUp":
          engineConnectPickerStore.moveUp();
          break;
        case "ArrowDown":
          engineConnectPickerStore.moveDown();
          break;
        case "Enter":
          engineConnectPickerStore.confirm();
          break;
        case "Escape":
          engineConnectPickerStore.close();
          break;
      }
      return;
    }

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

    // Esc: 대기 중이던 시퀀스 버퍼를 비움(게임 화면에서만 - 설정 화면 닫기는 위에서
    // 이미 처리됨). Esc는 keybindingsStore가 어떤 액션에도 배정을 막아둔 예약된
    // 키라(RESERVED_KEY 참고) 그 외의 용도로 쓰일 일이 없으므로, 버퍼가 비어
    // 있어도 그냥 여기서 끝낸다(아래 dispatch(key)로 넘길 필요가 없음).
    if (key === "Escape") {
      clearPrefix();
      evt.preventDefault();
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
