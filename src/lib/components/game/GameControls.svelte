<script lang="ts">
  // 버튼 패널: [착수 확정]/[취소] 전환, 다음 착수 색 전환, 대국 모드 전환 등 (설정 진입은 TopBar로 이동)
  // portrait에서는 하단 가로 바, landscape/desktop에서는 우측 세로 컬럼(GameScreen 레이아웃이 방향 전환)
  import { t } from "../../i18n";
  import { boardStore } from "../../stores/board.svelte";
  import stoneBlackUrl from "../../../assets/stone-black.png";
  import stoneWhiteUrl from "../../../assets/stone-white.png";

  const stoneImages = { black: stoneBlackUrl, white: stoneWhiteUrl };
</script>

<div class="game-controls">
  <button
    class="color-toggle"
    type="button"
    title={t("game.switchColor")}
    aria-label={t("game.switchColor")}
    onclick={() => boardStore.toggleTurn()}
  >
    <img src={stoneImages[boardStore.currentTurn]} alt={boardStore.currentTurn} />
  </button>
  <button
    type="button"
    disabled={!boardStore.canGoBack}
    onclick={() => boardStore.goBack()}
  >
    {t("game.back")}
  </button>
  <!-- 이미 돌이 있는 칸을 가리키고 있을 때도 버튼 자체는 항상 그대로 표시하되,
  confirmMove()가 그 경우 아무 동작도 하지 않으므로(임의의 돌 제거 기능 폐지) 눌러도
  아무 효과가 없음 -->
  <button
    class="primary"
    type="button"
    disabled={!boardStore.pendingMove}
    onclick={() => boardStore.confirmMove()}
  >
    {t("game.confirmMove")}
  </button>
  <button
    type="button"
    disabled={!boardStore.pendingMove}
    onclick={() => boardStore.cancelPending()}
  >
    {t("game.cancel")}
  </button>
  <button
    class="danger"
    type="button"
    disabled={!boardStore.canGoBack}
    onclick={() => boardStore.removeLastMove()}
  >
    {t("game.removeLastMove")}
  </button>
</div>

<style>
  .game-controls {
    display: flex;
    flex-direction: row;
    flex-wrap: wrap;
    gap: 8px;
    width: 100%;
  }

  button {
    flex: 1 1 auto;
    min-width: 96px;
    padding: 10px 12px;
    border: 1px solid #444;
    border-radius: 6px;
    background: transparent;
    color: inherit;
  }

  button.primary {
    border-color: #2e8b57;
    background: #2e8b57;
    color: #fff;
  }

  button.danger {
    border-color: #a33;
    color: #d66;
  }

  button:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  /* 다음 착수 색 전환 버튼: 텍스트 버튼과 달리 늘어나지 않는 정사각형 아이콘 버튼 */
  button.color-toggle {
    flex: 0 0 auto;
    min-width: 0;
    width: 44px;
    height: 44px;
    padding: 4px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  button.color-toggle img {
    width: 100%;
    height: 100%;
    object-fit: contain;
    pointer-events: none;
  }
</style>
