<script lang="ts">
  // 이번 Phase(원격 엔진 SSH 연동)의 최종 검증용 콘솔: 임의 GTP 명령을 보내고 응답을
  // 확인. kata-analyze 스트리밍 이벤트도 함께 로그로 찍어서 실전 데이터로 파이프라인이
  // 살아있는지 눈으로 바로 확인할 수 있게 함. 바둑판 위 오버레이 렌더링은 Phase 3.
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { t } from "../../i18n";
  import { appErrorMessage } from "../../appError";
  import { connectionStore } from "../../stores/connection.svelte";
  import type { KataAnalyzeResult } from "../../generated/bindings";

  interface LogEntry {
    kind: "sent" | "received" | "analysis" | "error";
    text: string;
  }

  let command = $state("");
  let log = $state<LogEntry[]>([]);
  let sending = $state(false);

  listen<KataAnalyzeResult>("kata-analyze", (event) => {
    const top = event.payload.candidates[0];
    const text = top
      ? `info: ${event.payload.candidates.length} candidates, top ${top.move} winrate=${(top.winrate ?? 0).toFixed(3)} visits=${top.visits}`
      : "info: (empty)";
    log.push({ kind: "analysis", text });
  });

  async function send() {
    const cmd = command.trim();
    if (!cmd) return;
    log.push({ kind: "sent", text: cmd });
    sending = true;
    try {
      const response = await invoke<string>("send_gtp_command", { command: cmd });
      log.push({ kind: "received", text: response || "(empty response)" });
    } catch (e) {
      log.push({ kind: "error", text: appErrorMessage(e) });
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
    {#each log as entry, i (i)}
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
