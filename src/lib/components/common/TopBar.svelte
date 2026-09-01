<script lang="ts">
  // 상단 바: 앱 타이틀 + SSH/GTP 연결 상태 표시 + 설정 진입 버튼
  import { connectionStore } from "../../stores/connection.svelte";
  import { t } from "../../i18n";

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
    <button type="button" class="settings-button" onclick={onOpenSettings}>
      {t("settings.title")}
    </button>
  </div>
</header>

<style>
  .top-bar {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    /* Android 상태 바(노치/카메라 컷아웃 포함)와 겹치지 않도록 안전 영역만큼 위쪽 여백 추가 */
    padding-top: calc(8px + env(safe-area-inset-top, 0px));
    gap: 8px;
  }

  .title {
    font-weight: 600;
  }

  .right {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .status {
    font-size: 0.85rem;
    opacity: 0.8;
  }

  .settings-button {
    padding: 4px 10px;
    border: 1px solid #444;
    border-radius: 6px;
    background: transparent;
    color: inherit;
    font-size: 0.85rem;
  }

  .status-connected {
    color: #2e8b57;
  }

  .status-connecting,
  .status-reconnecting {
    color: #d9a441;
  }

  .status-error {
    color: #c0392b;
  }
</style>
