<script lang="ts">
  // 등록된 서버 프로필(host/port/username/key) 목록 및 연결/전환 UI
  import { t } from "../../i18n";
  import { serverProfilesStore } from "../../stores/serverProfiles.svelte";
  import { connectionStore } from "../../stores/connection.svelte";

  serverProfilesStore.refresh();

  const statusKey = {
    disconnected: "status.disconnected",
    connecting: "status.connecting",
    connected: "status.connected",
    reconnecting: "status.reconnecting",
    error: "status.error",
  } as const;

  let connectingId = $state<string | null>(null);

  async function handleConnect(id: string) {
    connectingId = id;
    try {
      await connectionStore.connect(id);
      await serverProfilesStore.setActive(id);
    } catch {
      // 실패 원인은 connectionStore.lastError로 노출됨
    } finally {
      connectingId = null;
    }
  }

  async function handleDisconnect() {
    await connectionStore.disconnect();
  }
</script>

<ul class="server-profile-list">
  {#each serverProfilesStore.profiles as profile (profile.id)}
    <li class="profile-item">
      <div class="profile-info">
        <span class="profile-name">{profile.name || profile.host}</span>
        <span class="profile-detail"
          >{profile.username}@{profile.host}:{profile.port}</span
        >
        {#if profile.hasPassphrase}
          <span class="passphrase-warning">{t("settings.passphraseWarning")}</span>
        {/if}
      </div>
      <div class="profile-actions">
        <button
          type="button"
          onclick={() => handleConnect(profile.id)}
          disabled={profile.hasPassphrase || connectingId === profile.id}
        >
          {serverProfilesStore.activeProfileId === profile.id &&
          connectionStore.status === "connected"
            ? t("settings.connected")
            : t("settings.connect")}
        </button>
        <button
          type="button"
          class="danger"
          onclick={() => serverProfilesStore.remove(profile.id)}
        >
          {t("settings.delete")}
        </button>
      </div>
    </li>
  {:else}
    <li class="empty">{t("settings.noProfiles")}</li>
  {/each}
</ul>

{#if connectionStore.status !== "disconnected"}
  <div class="connection-status status-{connectionStore.status}">
    <span>{t(statusKey[connectionStore.status])}</span>
    <button type="button" onclick={handleDisconnect}>
      {t("settings.disconnect")}
    </button>
  </div>
  {#if connectionStore.lastError}
    <p class="error">{connectionStore.lastError}</p>
  {/if}
{/if}

<style>
  .server-profile-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .profile-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 8px;
    border: 1px solid #444;
    border-radius: 6px;
  }

  .profile-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .profile-name {
    font-weight: 600;
  }

  .profile-detail {
    font-size: 0.8rem;
    opacity: 0.8;
  }

  .passphrase-warning {
    font-size: 0.75rem;
    color: #d9a441;
  }

  .profile-actions {
    display: flex;
    gap: 6px;
    flex-shrink: 0;
  }

  .profile-actions button {
    padding: 6px 10px;
    border: 1px solid #444;
    border-radius: 4px;
    background: transparent;
    color: inherit;
    font-size: 0.8rem;
  }

  .profile-actions button.danger {
    border-color: #a33;
    color: #d66;
  }

  .empty {
    opacity: 0.7;
    font-size: 0.85rem;
  }

  .connection-status {
    margin-top: 8px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 6px 8px;
    border-radius: 6px;
    border: 1px solid #444;
  }

  .error {
    margin: 4px 0 0;
    color: #c0392b;
    font-size: 0.85rem;
  }
</style>
