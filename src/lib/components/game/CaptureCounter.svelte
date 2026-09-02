<script lang="ts">
  // 화면 맨 오른쪽 아래에 떠 있는 포로(따낸 돌) 수 배지. 값은 boardStore.captures를
  // 그대로 읽기만 함 - 실제 계산은 Rust GameTree가 매 착수(따내기 판정 포함)마다
  // 노드에 누적해 두고(game/mod.rs::Captures), confirmMove/board-updated로 오는
  // BoardSnapshot에 이미 반영되어 있으므로 여기서는 반응형으로 표시만 하면 즉시
  // 갱신된다 - 흑/백이 둘 다 자동 착수 중이라 여러 수가 연달아 반영될 때도
  // board-updated 이벤트가 매 수마다 오므로(board.svelte.ts 참고) 최종 결과만
  // 반영되는 게 아니라 잡는 순간순간 갱신됨.
  //
  // GameScreen 전체를 기준으로 position: fixed로 화면 맨 오른쪽 아래 구석에 고정 -
  // board-area/control-panel(세로/가로 방향에 따라 레이아웃이 바뀜)에 속하지 않는
  // 완전히 독립된 오버레이라 어느 방향이든 항상 같은 화면 구석에 남아 있음.
  // pointer-events: none이라 그 밑에 있는 버튼 클릭을 가리지 않음.
  import { boardStore } from "../../stores/board.svelte";
  import { t } from "../../i18n";
  import stoneBlackUrl from "../../../assets/stone-black.png";
  import stoneWhiteUrl from "../../../assets/stone-white.png";
</script>

<div class="capture-counter" title={t("game.captures")}>
  <span class="entry">
    <img src={stoneBlackUrl} alt="black" />
    {boardStore.captures.black}
  </span>
  <span class="entry">
    <img src={stoneWhiteUrl} alt="white" />
    {boardStore.captures.white}
  </span>
</div>

<style>
  .capture-counter {
    position: fixed;
    right: calc(10px + env(safe-area-inset-right, 0px));
    bottom: calc(10px + env(safe-area-inset-bottom, 0px));
    z-index: 20;
    display: flex;
    gap: 12px;
    padding: 6px 12px;
    border-radius: 8px;
    background: rgba(0, 0, 0, 0.6);
    color: #fff;
    font-size: 0.95rem;
    font-weight: bold;
    pointer-events: none;
  }

  .entry {
    display: flex;
    align-items: center;
    gap: 5px;
  }

  .entry img {
    width: 18px;
    height: 18px;
    object-fit: contain;
  }
</style>
