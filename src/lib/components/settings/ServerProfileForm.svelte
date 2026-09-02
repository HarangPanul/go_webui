<script lang="ts">
  // 서버 프로필 등록/수정 폼 (name, host, port, username, engineCommand, key)
  import { t } from "../../i18n";
  import { serverProfilesStore } from "../../stores/serverProfiles.svelte";
  import SshKeyInput from "./SshKeyInput.svelte";

  // onDone: 저장 성공 또는 취소로 폼이 "끝났을 때" 호출 - SettingsScreen이 이 폼을
  // "엔진 추가" 버튼 뒤로 숨길 때 사용(폼 자체는 계속 열어둘지 닫을지 모르므로 부모가
  // 결정). 프로필 목록에서 "수정"을 눌러 편집 모드로 들어온 경우에도 저장/취소하면
  // 마찬가지로 다시 접힌 상태로 돌아가야 자연스러움.
  let { onDone }: { onDone?: () => void } = $props();

  const DEFAULT_ENGINE_COMMAND = "katago gtp";

  let editingId = $state<string | null>(null);
  let name = $state("");
  let host = $state("");
  let port = $state(22);
  let username = $state("");
  let privateKey = $state("");
  let engineCommand = $state(DEFAULT_ENGINE_COMMAND);
  let saving = $state(false);
  let error = $state<string | null>(null);

  // 목록에서 수정 버튼을 누르면 store.editingProfile이 채워짐 -> 폼에 반영.
  // key 원문은 보안상 프론트로 절대 내려오지 않으므로 항상 빈 칸으로 두고,
  // 비워둔 채로 저장하면 백엔드가 기존 key를 유지한다.
  $effect(() => {
    const profile = serverProfilesStore.editingProfile;
    if (!profile) return;
    editingId = profile.id;
    name = profile.name;
    host = profile.host;
    port = profile.port;
    username = profile.username;
    privateKey = "";
    engineCommand = profile.engineCommand;
  });

  function resetForm() {
    editingId = null;
    name = "";
    host = "";
    port = 22;
    username = "";
    privateKey = "";
    engineCommand = DEFAULT_ENGINE_COMMAND;
  }

  function handleCancel() {
    serverProfilesStore.cancelEdit();
    resetForm();
    onDone?.();
  }

  async function handleSubmit(event: SubmitEvent) {
    event.preventDefault();
    saving = true;
    error = null;
    try {
      await serverProfilesStore.save({
        id: editingId ?? undefined,
        name,
        host,
        port,
        username,
        privateKey,
        engineCommand,
      });
      resetForm();
      onDone?.();
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
    {#if editingId}
      <span class="hint">{t("settings.sshKeyEditHint")}</span>
    {/if}
  </label>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <div class="form-actions">
    <button type="submit" class="primary" disabled={saving}>
      {editingId ? t("settings.saveChanges") : t("settings.saveProfile")}
    </button>
    <button type="button" onclick={handleCancel} disabled={saving}>
      {t("settings.cancelEdit")}
    </button>
  </div>
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

  .form-actions {
    display: flex;
    gap: 8px;
  }

  .form-actions button {
    padding: 8px 16px;
    border: 1px solid #444;
    border-radius: 6px;
    background: transparent;
    color: inherit;
  }

  button.primary {
    align-self: flex-start;
    border: 1px solid #2e8b57;
    background: #2e8b57;
    color: #fff;
  }

  button.primary:disabled {
    opacity: 0.5;
  }

  .hint {
    font-size: 0.75rem;
    opacity: 0.7;
  }

  .error {
    margin: 0;
    color: #c0392b;
    font-size: 0.85rem;
  }
</style>
