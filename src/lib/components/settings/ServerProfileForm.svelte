<script lang="ts">
  // 서버 프로필 등록/수정 폼 (name, host, port, username, engineCommand, key)
  import { t } from "../../i18n";
  import { serverProfilesStore } from "../../stores/serverProfiles.svelte";
  import SshKeyInput from "./SshKeyInput.svelte";

  const DEFAULT_ENGINE_COMMAND = "katago gtp";

  let name = $state("");
  let host = $state("");
  let port = $state(22);
  let username = $state("");
  let privateKey = $state("");
  let engineCommand = $state(DEFAULT_ENGINE_COMMAND);
  let saving = $state(false);
  let error = $state<string | null>(null);

  async function handleSubmit(event: SubmitEvent) {
    event.preventDefault();
    saving = true;
    error = null;
    try {
      await serverProfilesStore.save({
        name,
        host,
        port,
        username,
        privateKey,
        engineCommand,
      });
      name = "";
      host = "";
      port = 22;
      username = "";
      privateKey = "";
      engineCommand = DEFAULT_ENGINE_COMMAND;
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }
</script>

<form class="server-profile-form" onsubmit={handleSubmit}>
  <label>
    {t("settings.profileName")}
    <input type="text" bind:value={name} required />
  </label>
  <label>
    {t("settings.host")}
    <input type="text" bind:value={host} required />
  </label>
  <label>
    {t("settings.port")}
    <input type="number" min="1" max="65535" bind:value={port} required />
  </label>
  <label>
    {t("settings.username")}
    <input type="text" bind:value={username} required />
  </label>
  <label>
    {t("settings.engineCommand")}
    <input type="text" bind:value={engineCommand} />
  </label>
  <label>
    {t("settings.sshKey")}
    <SshKeyInput bind:value={privateKey} />
  </label>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <button type="submit" class="primary" disabled={saving}>
    {t("settings.saveProfile")}
  </button>
</form>

<style>
  .server-profile-form {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 0.85rem;
  }

  input {
    padding: 6px 8px;
    border: 1px solid #444;
    border-radius: 4px;
    background: transparent;
    color: inherit;
  }

  button.primary {
    align-self: flex-start;
    padding: 8px 16px;
    border: 1px solid #2e8b57;
    border-radius: 6px;
    background: #2e8b57;
    color: #fff;
  }

  button.primary:disabled {
    opacity: 0.5;
  }

  .error {
    margin: 0;
    color: #c0392b;
    font-size: 0.85rem;
  }
</style>
