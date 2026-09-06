<script lang="ts">
  // 서버 프로필 등록/전환 + SSH Key 입력 + GTP 콘솔 + 키보드 단축키 화면.
  // 서버 프로필(목록+추가 버튼)만 최상단에 항상 펼쳐두고, 나머지(대국 설정/키보드
  // 단축키/GTP 콘솔)는 CollapsibleSection으로 각각 접어 넣어 화면이 한 번에 너무
  // 길게 늘어지지 않게 함 - ServerProfileForm이 "엔진 추가" 버튼 뒤에 숨는 것과
  // 같은 패턴을 영역별로 확장한 것.
  import { t } from "../i18n";
  import ServerProfileList from "../components/settings/ServerProfileList.svelte";
  import ServerProfileForm from "../components/settings/ServerProfileForm.svelte";
  import GtpConsole from "../components/settings/GtpConsole.svelte";
  import KeybindingsForm from "../components/settings/KeybindingsForm.svelte";
  import KomiForm from "../components/settings/KomiForm.svelte";
  import MaxVisitsForm from "../components/settings/MaxVisitsForm.svelte";
  import InstantMoveForm from "../components/settings/InstantMoveForm.svelte";
  import PendingCrosshairForm from "../components/settings/PendingCrosshairForm.svelte";
  import LanguageForm from "../components/settings/LanguageForm.svelte";
  import CollapsibleSection from "../components/settings/CollapsibleSection.svelte";
  import { serverProfilesStore } from "../stores/serverProfiles.svelte";
  import { platformStore } from "../stores/platform.svelte";
  import { handleRovingArrowKeys } from "../utils/rovingFocus";

  let { onClose }: { onClose: () => void } = $props();

  // 위/아래 화살표로 화면 안 버튼/입력 사이를 이동(rovingFocus.ts 참고) - 마우스나
  // 터치 없이도 설정 화면을 조작할 수 있게 함. 숫자 입력(komi/port 등)에서 화살표를
  // 누르면 브라우저 기본 동작(스피너로 값 증가/감소)이 이 이동으로 대체되므로 값이
  // 바뀌지 않는다. 하드코딩된 요소 목록이 아니라 매번 DOM을 다시 훑으므로, 이후에
  // 이 화면에 새 버튼/입력이 추가돼도 자동으로 이 탐색에 포함된다.
  let rootEl: HTMLElement | undefined = $state();

  function onKeyDown(evt: KeyboardEvent) {
    if (!rootEl) return;
    handleRovingArrowKeys(evt, rootEl);
  }

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

<section class="settings-screen" bind:this={rootEl} onkeydown={onKeyDown}>
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

  <CollapsibleSection title={t("settings.gameSettings")}>
    <KomiForm />
    <!-- max_visits는 로컬 온디바이스 엔진(tauri-plugin-katago-local, Android 전용)의
         GTP 확장 명령이라 그 기능이 없는 플랫폼에 실효 없는 컨트롤을 보여주지 않게
         gating한다 - ServerProfileForm의 "Local" 프로필 선택지와 같은 기준
         (platformStore.supportsLocalEngine). -->
    {#if platformStore.supportsLocalEngine}
      <MaxVisitsForm />
    {/if}
    <InstantMoveForm />
    <PendingCrosshairForm />
  </CollapsibleSection>

  <CollapsibleSection title={t("settings.language")}>
    <LanguageForm />
  </CollapsibleSection>

  <CollapsibleSection title={t("settings.keybindings.title")}>
    <KeybindingsForm />
  </CollapsibleSection>

  <CollapsibleSection title={t("settings.gtpConsole")}>
    <GtpConsole />
  </CollapsibleSection>

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
