<script lang="ts">
  // 서버 프로필 등록/전환 + SSH Key 입력 + GTP 콘솔 + 키보드 단축키 화면
  import { t } from "../i18n";
  import ServerProfileList from "../components/settings/ServerProfileList.svelte";
  import ServerProfileForm from "../components/settings/ServerProfileForm.svelte";
  import GtpConsole from "../components/settings/GtpConsole.svelte";
  import KeybindingsForm from "../components/settings/KeybindingsForm.svelte";

  let { onClose }: { onClose: () => void } = $props();
</script>

<section class="settings-screen">
  <h2>{t("settings.serverProfile")}</h2>
  <ServerProfileList />
  <ServerProfileForm />

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
