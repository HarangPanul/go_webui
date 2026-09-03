<script lang="ts">
  // 외부 파일 탐색기 대신 SSH Private Key 텍스트 붙여넣기 입력
  // passphrase 걸린 key 감지는 클라이언트 측 휴리스틱(-----BEGIN ENCRYPTED)으로 즉시
  // 경고만 표시 (1차 버전 범위). 최종 판단은 서버의 hasPassphrase 응답.
  //
  // 데스크탑(Linux/macOS/Windows)에서는 ~/.ssh를 스캔해 감지한 key를 목록에서 골라
  // 바로 채워 넣을 수도 있음(list_local_ssh_keys/load_local_ssh_key, desktop 전용
  // Rust 커맨드). Android는 "시스템 SSH key"라는 개념이 없어 list_local_ssh_keys가
  // 항상 에러를 반환하므로, 그 경우 이 감지 UI 자체를 그냥 숨긴다 - 붙여넣기 입력은
  // 모든 플랫폼에서 그대로 동작.
  import { invoke } from "@tauri-apps/api/core";
  import { t } from "../../i18n";
  import { appErrorMessage } from "../../appError";
  import type { LocalSshKeyInfo } from "../../types/serverProfile";

  let { value = $bindable("") }: { value?: string } = $props();

  let looksEncrypted = $derived(value.includes("ENCRYPTED"));

  let localKeys = $state<LocalSshKeyInfo[]>([]);
  let selectedPath = $state("");
  let loadError = $state<string | null>(null);

  // 마운트 시 한 번 감지 시도. 모바일 등 미지원 플랫폼에서는 커맨드 자체가 에러를
  // 반환하므로 조용히 무시(목록이 빈 채로 남아 UI가 자동으로 숨겨짐).
  invoke<LocalSshKeyInfo[]>("list_local_ssh_keys")
    .then((keys) => (localKeys = keys))
    .catch(() => (localKeys = []));

  async function handleSelect(event: Event) {
    const path = (event.target as HTMLSelectElement).value;
    selectedPath = path;
    if (!path) return;
    loadError = null;
    try {
      value = await invoke<string>("load_local_ssh_key", { path });
    } catch (e) {
      loadError = appErrorMessage(e);
    }
  }
</script>

<div class="ssh-key-input">
  {#if localKeys.length > 0}
    <label class="detect-row">
      {t("settings.sshKeyDetected")}
      <select value={selectedPath} onchange={handleSelect}>
        <option value="">{t("settings.sshKeyDetectPlaceholder")}</option>
        {#each localKeys as key (key.path)}
          <option value={key.path} disabled={key.hasPassphrase}>
            {key.name}{key.hasPassphrase ? ` (${t("settings.passphraseWarning")})` : ""}
          </option>
        {/each}
      </select>
    </label>
  {/if}
  {#if loadError}
    <p class="error">{loadError}</p>
  {/if}
  <textarea
    bind:value
    rows="6"
    placeholder="-----BEGIN PRIVATE KEY-----"
    spellcheck="false"
  ></textarea>
  {#if looksEncrypted}
    <p class="warning">{t("settings.passphraseWarning")}</p>
  {/if}
</div>

<style>
  .ssh-key-input {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .detect-row {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 0.8rem;
  }

  select {
    padding: 6px 8px;
    border: 1px solid #444;
    border-radius: 4px;
    background: transparent;
    color: inherit;
  }

  textarea {
    width: 100%;
    padding: 6px 8px;
    border: 1px solid #444;
    border-radius: 4px;
    background: transparent;
    color: inherit;
    font-family: monospace;
    font-size: 0.75rem;
    resize: vertical;
  }

  .warning {
    margin: 0;
    color: #d9a441;
    font-size: 0.8rem;
  }

  .error {
    margin: 0;
    color: #c0392b;
    font-size: 0.8rem;
  }
</style>
