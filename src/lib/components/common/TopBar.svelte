<script lang="ts">
  // 상단 바: 앱 타이틀 + SSH/GTP 연결 상태 표시 + 설정 진입 버튼
  import { connectionStore } from "../../stores/connection.svelte";
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
</script>

<header class="top-bar">
  <span class="title">{t("app.title")}</span>
  <div class="right">
    <span class="status status-{connectionStore.status}">
      {t(statusKey[connectionStore.status])}
    </span>
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
