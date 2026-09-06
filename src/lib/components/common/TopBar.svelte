<script lang="ts">
  // 상단 바: 앱 타이틀 + SSH/GTP 연결 상태 표시 + 설정 진입 버튼.
  // 여러 프로필이 동시에 연결될 수 있으므로(connectionStore) 한 번이라도 연결을
  // 시도한(연결 안 됨이 아닌) 프로필은 각각 이름 칩으로 보여줘 "지금 어떤 엔진들이
  // 연결돼 있는지"를 한눈에 알 수 있게 한다 - 프로필별 세부 정보(host/port 등)는
  // 여전히 Settings의 ServerProfileList에서 확인. 시도한 프로필이 하나도 없으면
  // (아직 연결을 한 번도 안 해본 상태) 칩 대신 기존처럼 요약 상태 텍스트를 보여준다.
  import { connectionStore } from "../../stores/connection.svelte";
  import { serverProfilesStore } from "../../stores/serverProfiles.svelte";
  import { t } from "../../i18n";
  import Button from "../ui/Button.svelte";

  let { onOpenSettings }: { onOpenSettings: () => void } = $props();

  const statusKey = {
    disconnected: "status.disconnected",
    connecting: "status.connecting",
    connected: "status.connected",
    reconnecting: "status.reconnecting",
    error: "status.error",
  } as const;

  const activeProfiles = $derived(
    serverProfilesStore.profiles.filter(
      (p) => connectionStore.statusFor(p.id) !== "disconnected",
    ),
  );
</script>

<header class="top-bar">
  <span class="title">{t("app.title")}</span>
  <div class="right">
    {#if activeProfiles.length > 0}
      <ul class="engine-chips">
        {#each activeProfiles as profile (profile.id)}
          {@const status = connectionStore.statusFor(profile.id)}
          <li class="engine-chip status-{status}" title={t(statusKey[status])}>
            {profile.name || profile.host}
          </li>
        {/each}
      </ul>
    {:else}
      <span class="status status-{connectionStore.overallStatus}">
        {t(statusKey[connectionStore.overallStatus])}
      </span>
    {/if}
    <Button class="settings-button" onclick={onOpenSettings}>
      {t("settings.title")}
    </Button>
  </div>
</header>

<style>
  .top-bar {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-4) var(--space-6);
    /* Android 상태 바(노치/카메라 컷아웃 포함)와 겹치지 않도록 안전 영역만큼 위쪽 여백 추가 */
    padding-top: calc(var(--space-4) + env(safe-area-inset-top, 0px));
    gap: var(--space-4);
  }

  .title {
    font-weight: 600;
  }

  .right {
    display: flex;
    align-items: center;
    gap: var(--space-6);
  }

  .status {
    font-size: 0.85rem;
    opacity: 0.8;
  }

  .engine-chips {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-3);
  }

  .engine-chip {
    font-size: 0.75rem;
    padding: var(--space-1) var(--space-4);
    border: 1px solid currentColor;
    border-radius: 999px;
    opacity: 0.9;
    white-space: nowrap;
  }

  :global(.settings-button) {
    padding: var(--space-2) var(--space-5);
    font-size: 0.85rem;
  }

  .status-connected {
    color: var(--color-primary);
  }

  .status-connecting,
  .status-reconnecting {
    color: var(--color-warning);
  }

  .status-error {
    color: var(--color-danger);
  }
</style>
