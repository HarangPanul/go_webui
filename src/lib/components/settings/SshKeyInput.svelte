<script lang="ts">
  // 외부 파일 탐색기 대신 SSH Private Key 텍스트 붙여넣기 입력
  // passphrase 걸린 key 감지는 클라이언트 측 휴리스틱(-----BEGIN ENCRYPTED)으로 즉시
  // 경고만 표시 (1차 버전 범위). 최종 판단은 서버의 hasPassphrase 응답.
  import { t } from "../../i18n";

  let { value = $bindable("") }: { value?: string } = $props();

  let looksEncrypted = $derived(value.includes("ENCRYPTED"));
</script>

<div class="ssh-key-input">
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
</style>
