<script lang="ts">
  // 서버 프로필 등록/전환 + SSH Key 입력 + GTP 콘솔 + 키보드 단축키 화면
  import { t } from "../i18n";
  import ServerProfileList from "../components/settings/ServerProfileList.svelte";
  import ServerProfileForm from "../components/settings/ServerProfileForm.svelte";
  import GtpConsole from "../components/settings/GtpConsole.svelte";
  import KeybindingsForm from "../components/settings/KeybindingsForm.svelte";
  import KomiForm from "../components/settings/KomiForm.svelte";
  import { serverProfilesStore } from "../stores/serverProfiles.svelte";

  let { onClose }: { onClose: () => void } = $props();

  // SSH host/port/username/key를 입력하는 폼(ServerProfileForm)은 기본으로 접어두고
  // "엔진 추가" 버튼 뒤에 숨긴다 - 이미 등록된 목록(ServerProfileList)만 봐도 되는
  // 경우가 대부분이라 매번 긴 입력 폼이 펼쳐져 있을 필요가 없다. 목록에서 "수정"을
  // 누르면(serverProfilesStore.editingProfile이 채워짐) 그때도 폼을 자동으로 펼쳐야
  // 하므로 그 값도 함께 지켜본다.
  let showForm = $state(false);

  $effect(() => {
    if (serverProfilesStore.editingProfile) {
      showForm = true;
    }
  });

  function closeForm() {
    showForm = false;
  }
</script>

<section class="settings-screen">
  <h2>{t("settings.serverProfile")}</h2>
  <ServerProfileList />

  {#if showForm}
    <ServerProfileForm onDone={closeForm} />
  {:else}
    <button type="button" class="add-engine-button" onclick={() => (showForm = true)}>
      {t("settings.addEngine")}
    </button>
  {/if}

  <hr />
  <KomiForm />

  <hr />
  <GtpConsole />

  <hr />
  <KeybindingsForm />

  <button class="close-button" onclick={onClose}>Close</button>
</section>

<style>
  .settings-screen {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 12px;
    overflow-y: auto;
    /* Android 상태 바/제스처 내비게이션 바(노치 포함)에 가려지지 않도록 안전
       영역만큼 위/아래 여백 추가 - 이 화면은 TopBar 없이 최상위에서 바로
       렌더링되므로 여기서 직접 처리해야 함 */
    padding-top: calc(12px + env(safe-area-inset-top, 0px));
    padding-bottom: calc(12px + env(safe-area-inset-bottom, 0px));
  }

  h2 {
    margin: 0;
    font-size: 1rem;
  }

  hr {
    border: none;
    border-top: 1px solid #444;
    margin: 4px 0;
    width: 100%;
  }

  .add-engine-button {
    align-self: flex-start;
    padding: 8px 16px;
    border: 1px solid #2e8b57;
    border-radius: 6px;
    background: transparent;
    color: #2e8b57;
  }

  .close-button {
    align-self: flex-start;
    padding: 14px 24px;
    font-size: 1.1rem;
    border: 1px solid #444;
    border-radius: 8px;
    background: transparent;
    color: inherit;
  }
</style>
