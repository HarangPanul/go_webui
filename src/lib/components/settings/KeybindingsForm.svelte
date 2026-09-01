<script lang="ts">
  // 착수 확정/색 전환/뒤로 가기/마지막 수 제거 단축키를 원하는 키로 재배정하는 UI.
  // "키 변경" 버튼을 누르면 다음에 누르는 키 하나가 그 액션에 등록됨 (Esc로 취소).
  import { t } from "../../i18n";
  import { keybindingsStore, type KeyAction } from "../../stores/keybindings.svelte";

  const ACTIONS: { action: KeyAction; labelKey: "game.confirmMove" | "game.switchColor" | "game.back" | "game.removeLastMove" }[] = [
    { action: "confirmMove", labelKey: "game.confirmMove" },
    { action: "changeColor", labelKey: "game.switchColor" },
    { action: "back", labelKey: "game.back" },
    { action: "removeLastMove", labelKey: "game.removeLastMove" },
  ];

  // 지금 다음 키 입력을 기다리고 있는 액션 (없으면 null)
  let listeningFor = $state<KeyAction | null>(null);

  // 표시용 키 이름: Space는 눈에 보이지 않으므로 텍스트로 풀어서 보여줌
  function displayKey(key: string): string {
    if (!key) return t("settings.keybindings.unset");
    if (key === " ") return "Space";
    return key;
  }

  function startListening(action: KeyAction) {
    listeningFor = action;
  }

  // capture 단계에서 가로채므로 BoardCanvas의 window keydown 핸들러(기존 키 동작)보다
  // 먼저 실행되고, stopPropagation으로 그쪽까지 전달되는 것도 막음
  function handleCaptureKeydown(evt: KeyboardEvent) {
    if (!listeningFor) return;
    evt.preventDefault();
    evt.stopPropagation();

    if (evt.key === "Escape") {
      listeningFor = null;
      return;
    }
    // 조합키(Ctrl/Alt/Meta)는 단축키로 쓰지 않음
    if (evt.ctrlKey || evt.altKey || evt.metaKey) return;

    keybindingsStore.setKey(listeningFor, evt.key);
    listeningFor = null;
  }

  $effect(() => {
    if (!listeningFor) return;
    window.addEventListener("keydown", handleCaptureKeydown, true);
    return () => window.removeEventListener("keydown", handleCaptureKeydown, true);
  });
</script>

<div class="keybindings-form">
  <h3>{t("settings.keybindings.title")}</h3>
  {#each ACTIONS as { action, labelKey } (action)}
    <div class="row">
      <span class="label">{t(labelKey)}</span>
      <button
        type="button"
        class="key"
        class:listening={listeningFor === action}
        onclick={() => startListening(action)}
      >
        {listeningFor === action
          ? t("settings.keybindings.pressKey")
          : displayKey(keybindingsStore.keyFor(action))}
      </button>
      <button
        type="button"
        class="reset"
        title={t("settings.keybindings.reset")}
        aria-label={t("settings.keybindings.reset")}
        onclick={() => keybindingsStore.resetToDefault(action)}
      >
        ↺
      </button>
    </div>
  {/each}
</div>

<style>
  .keybindings-form {
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 100%;
  }

  h3 {
    margin: 0;
    font-size: 1em;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .label {
    flex: 1 1 auto;
  }

  button.key {
    min-width: 96px;
    padding: 6px 10px;
    border: 1px solid #444;
    border-radius: 6px;
    background: transparent;
    color: inherit;
  }

  button.key.listening {
    border-color: #2e8b57;
    color: #2e8b57;
  }

  button.reset {
    padding: 4px 8px;
    border: 1px solid #444;
    border-radius: 6px;
    background: transparent;
    color: inherit;
  }
</style>
