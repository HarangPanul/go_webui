<script lang="ts">
  // "ec" 단축키로 뜨는 엔진 선택창. 실제 위/아래 화살표·Enter·Esc 키 입력은
  // KeyboardShortcuts.svelte가 (다른 모든 단축키와 같은 리스너에서) 받아
  // engineConnectPickerStore를 조작하고, 이 컴포넌트는 그 결과를 그대로
  // 그려서 보여주기만 한다 - 마우스로 항목을 직접 클릭하는 것만 여기서 처리.
  import { t } from "../../i18n";
  import { serverProfilesStore } from "../../stores/serverProfiles.svelte";
  import { connectionStore } from "../../stores/connection.svelte";
  import { engineConnectPickerStore } from "../../stores/engineConnectPicker.svelte";
  import Button from "../ui/Button.svelte";

  const statusKey = {
    disconnected: "status.disconnected",
    connecting: "status.connecting",
    connected: "status.connected",
    reconnecting: "status.reconnecting",
    error: "status.error",
  } as const;
</script>

{#if engineConnectPickerStore.isOpen}
  <!-- role="presentation": 배경을 눌러 닫는 건 마우스 전용 편의 제스처일 뿐이고,
  같은 동작(닫기)은 Esc 전역 단축키로 키보드에서도 이미 가능하다(위 주석 참고). -->
  <div class="backdrop" role="presentation" onclick={() => engineConnectPickerStore.close()}>
    <div class="picker">
      <span class="picker-title">{t("game.engineConnect")}</span>
      <ul>
        {#each serverProfilesStore.profiles as profile, index (profile.id)}
          {@const status = connectionStore.statusFor(profile.id)}
          <li>
            <Button
              class="picker-item status-{status}"
              active={index === engineConnectPickerStore.selectedIndex}
              onclick={(evt) => {
                evt.stopPropagation();
                engineConnectPickerStore.confirm(index);
              }}
            >
              <span class="name">{profile.name || profile.host}</span>
              <span class="status-label">{t(statusKey[status])}</span>
            </Button>
          </li>
        {/each}
      </ul>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 30;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.5);
  }

  .picker {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    min-width: 220px;
    max-width: min(90vw, 360px);
    max-height: 70vh;
    padding: var(--space-6);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    /* Panel.svelte 등 나머지 UI는 배경을 투명하게 두고 페이지 배경을 그대로
       비치게 하지만, 이 창은 배경 위에 떠 있는 오버레이라 그 아래 내용이 그대로
       비치면 글자가 묻힌다 - CaptureCounter.svelte와 같은 이유로 테마와 무관하게
       고정된 어두운 배경 + 밝은 글자색을 쓴다. */
    background: rgba(20, 20, 20, 0.95);
    color: #fff;
  }

  .picker-title {
    font-weight: 600;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    overflow-y: auto;
  }

  :global(.picker-item) {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
  }

  .status-label {
    font-size: 0.75rem;
    opacity: 0.8;
  }

  :global(.picker-item.status-connected) .status-label {
    color: var(--color-primary);
  }

  :global(.picker-item.status-connecting) .status-label,
  :global(.picker-item.status-reconnecting) .status-label {
    color: var(--color-warning);
  }

  :global(.picker-item.status-error) .status-label {
    color: var(--color-danger);
  }
</style>
