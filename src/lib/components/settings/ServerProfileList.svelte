<script lang="ts">
  // 등록된 서버 프로필(host/port/username/key) 목록 및 연결/전환 UI.
  // 여러 프로필을 동시에 연결해둘 수 있으므로 각 항목이 자기 자신의 연결 상태/버튼을
  // 독립적으로 갖는다 - 흑/백 중 뭘 둘지는 여기서 정하지 않고(메인 화면 GameControls
  // 참고) "연결되어 있는지"만 다룬다.
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

  // 연결 요청이 오가는 동안(SSH 핸드셰이크 등) 같은 프로필의 Connect 버튼을 다시
  // 누르면 아직 sessions 맵에 안 잡힌 상태라 중복 연결 방지 로직(connection_service::
  // connect의 is_connected 체크)을 통과해버려 SSH 연결이 두 번 열릴 수 있다 - 그래서
  // 요청이 끝날 때까지 프로필별로 버튼을 비활성화해둔다.
  let connectingIds = $state<Set<string>>(new Set());

  async function handleConnect(id: string) {
    connectingIds = new Set(connectingIds).add(id);
    try {
      await connectionStore.connect(id);
      await serverProfilesStore.setActive(id);
    } catch {
      // 실패 원인은 connectionStore.errorFor(id)로 노출됨
    } finally {
      const next = new Set(connectingIds);
      next.delete(id);
      connectingIds = next;
    }
  }

  async function handleDisconnect(id: string) {
    await connectionStore.disconnect(id);
  }
</script>

<ul class="server-profile-list">
  {#each serverProfilesStore.profiles as profile (profile.id)}
    {@const status = connectionStore.statusFor(profile.id)}
    <li>
      <Panel class="profile-item">
        <div class="profile-info">
          <span class="profile-name">{profile.name || profile.host}</span>
          {#if profile.kind === "local"}
            <span class="profile-detail">{t("settings.profileKindLocal")}</span>
          {:else}
            <span class="profile-detail"
              >{profile.username}@{profile.host}:{profile.port}</span
            >
          {/if}
          {#if profile.hasPassphrase}
            <span class="passphrase-warning">{t("settings.passphraseWarning")}</span>
          {/if}
          {#if status !== "disconnected"}
            <span class="connection-status status-{status}">{t(statusKey[status])}</span>
          {/if}
          {#if connectionStore.errorFor(profile.id)}
            <span class="error">{connectionStore.errorFor(profile.id)}</span>
          {/if}
        </div>
        <div class="profile-actions">
          {#if status === "connected" || status === "connecting" || status === "reconnecting"}
            <Button onclick={() => handleDisconnect(profile.id)}>
              {t("settings.disconnect")}
            </Button>
          {:else}
            <Button
              onclick={() => handleConnect(profile.id)}
              disabled={profile.hasPassphrase || connectingIds.has(profile.id)}
            >
              {t("settings.connect")}
            </Button>
          {/if}
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

  .connection-status {
    font-size: 0.75rem;
    opacity: 0.8;
  }

  .connection-status.status-connected {
    color: var(--color-primary);
  }

  .connection-status.status-connecting,
  .connection-status.status-reconnecting {
    color: var(--color-warning);
  }

  .connection-status.status-error {
    color: var(--color-danger);
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

  .error {
    margin: 0;
    color: var(--color-danger);
    font-size: 0.75rem;
  }
</style>
