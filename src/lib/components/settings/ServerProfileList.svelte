<script lang="ts">
  // 등록된 서버 프로필(host/port/username/key) 목록 및 연결/전환 UI
  import { t } from "../../i18n";
  import { serverProfilesStore } from "../../stores/serverProfiles.svelte";
  import { connectionStore } from "../../stores/connection.svelte";
  import Button from "../ui/Button.svelte";
  import Panel from "../ui/Panel.svelte";

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
    <li>
      <Panel class="profile-item">
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
          <Button
            onclick={() => handleConnect(profile.id)}
            disabled={profile.hasPassphrase || connectingId === profile.id}
          >
            {serverProfilesStore.activeProfileId === profile.id &&
            connectionStore.status === "connected"
              ? t("settings.connected")
              : t("settings.connect")}
          </Button>
          <Button onclick={() => serverProfilesStore.startEdit(profile)}>
            {t("settings.edit")}
          </Button>
          <Button variant="danger" onclick={() => serverProfilesStore.remove(profile.id)}>
            {t("settings.delete")}
          </Button>
        </div>
      </Panel>
    </li>
  {:else}
    <li class="empty">{t("settings.noProfiles")}</li>
  {/each}
</ul>

{#if connectionStore.status !== "disconnected"}
  <Panel class="connection-status status-{connectionStore.status}">
    <span>{t(statusKey[connectionStore.status])}</span>
    <Button onclick={handleDisconnect}>
      {t("settings.disconnect")}
    </Button>
  </Panel>
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
    gap: var(--space-4);
  }

  /* 테두리/둥근 모서리/패딩은 ui/Panel.svelte가 담당 - 여기서는 이 목록 항목
     특유의 가로 배치만 지정 */
  :global(.profile-item) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
  }

  .profile-info {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
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
    color: var(--color-warning);
  }

  .profile-actions {
    display: flex;
    gap: var(--space-3);
    flex-shrink: 0;
  }

  /* 목록 안 버튼은 본문보다 작게(좁은 패딩 + 작은 글자 크기) */
  .profile-actions :global(.ui-button) {
    padding: var(--space-3) var(--space-5);
    font-size: 0.8rem;
  }

  .empty {
    opacity: 0.7;
    font-size: 0.85rem;
  }

  :global(.connection-status) {
    margin-top: var(--space-4);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
  }

  .error {
    margin: var(--space-2) 0 0;
    color: var(--color-danger);
    font-size: 0.85rem;
  }
</style>
