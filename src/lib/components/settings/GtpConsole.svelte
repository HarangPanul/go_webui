<script lang="ts">
  // 이번 Phase(원격 엔진 SSH 연동)의 최종 검증용 콘솔: 임의 GTP 명령을 보내고 응답을
  // 확인. kata-analyze 스트리밍 이벤트도 함께 로그로 찍어서 실전 데이터로 파이프라인이
  // 살아있는지 눈으로 바로 확인할 수 있게 함. 바둑판 위 오버레이 렌더링은 Phase 3.
  //
  // 실제 invoke()/이벤트 구독은 전부 gtpStore(stores/gtp.svelte.ts)가 담당 - 이
  // 컴포넌트는 그 결과(log)를 보여주고 입력을 store로 전달하기만 하는 순수 프레젠테이션.
  import { t } from "../../i18n";
  import { connectionStore } from "../../stores/connection.svelte";
  import { gtpStore } from "../../stores/gtp.svelte";

  let command = $state("");
  let sending = $state(false);

  async function send() {
    const cmd = command.trim();
    if (!cmd) return;
    sending = true;
    try {
      await gtpStore.send(cmd);
    } finally {
      sending = false;
      command = "";
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();
      send();
    }
  }

  let disabled = $derived(connectionStore.status !== "connected" || sending);
</script>

<div class="gtp-console">
  <div class="log">
    {#each gtpStore.log as entry, i (i)}
      <p class="log-entry {entry.kind}">{entry.text}</p>
    {/each}
  </div>
  <div class="input-row">
    <input
      type="text"
      bind:value={command}
      onkeydown={handleKeydown}
      disabled={disabled}
      placeholder="name"
    />
    <button type="button" onclick={send} disabled={disabled || !command.trim()}>
      {t("settings.send")}
    </button>
  </div>
</div>

<style>
  .gtp-console {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .log {
    max-height: 200px;
    overflow-y: auto;
    border: 1px solid #444;
    border-radius: 4px;
    padding: 6px 8px;
    font-family: monospace;
    font-size: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .log-entry {
    margin: 0;
    white-space: pre-wrap;
  }

  .log-entry.sent::before {
    content: "> ";
    opacity: 0.6;
  }

  .log-entry.received {
    opacity: 0.85;
  }

  .log-entry.analysis {
    color: #2e8b57;
  }

  .log-entry.error {
    color: #c0392b;
  }

  .input-row {
    display: flex;
    gap: 6px;
  }

  .input-row input {
    flex: 1;
    padding: 6px 8px;
    border: 1px solid #444;
    border-radius: 4px;
    background: transparent;
    color: inherit;
    font-family: monospace;
  }

  .input-row button {
    padding: 6px 12px;
    border: 1px solid #444;
    border-radius: 4px;
    background: transparent;
    color: inherit;
  }
</style>
