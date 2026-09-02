<script lang="ts">
  // 실시간 승률 막대: kata-analyze 스트리밍 결과(analysisStore)를 흑/백 비율 막대로
  // 표시. 세로 모드에서는 바둑판 바로 아래 가로 막대(흑이 오른쪽), 가로 모드/
  // 데스크탑에서는 바둑판 바로 오른쪽에 세로 막대(흑이 위쪽)로 배치 -
  // GameScreen.svelte의 board-area(바둑판을 담는 컨테이너) 안에서 GoBoard와 나란히
  // 놓여 방향 전환에 자연스럽게 맞물린다. 버튼 패널과는 무관하게 항상 바둑판에 붙어
  // 있음. GoBoard와 마찬가지로 폭/높이를 board-area 기준 cqmin(컨테이너의 짧은 쪽)으로
  // 맞춰서 - board-area의 남는 공간에 비례한 %가 아니라 - 바둑판 정사각형의 실제 한
  // 변 길이와 정확히 같아지게 함. 그래야 막대가 바둑판 가장자리에 여백 없이 맞닿는다.
  //
  // kata-analyze의 winrate 필드는 "분석을 시작한 시점에 둘 차례였던 색" 기준 승률이다.
  // analysisStore가 result/forColor를 항상 같은 쌍으로 주므로(analysis.svelte.ts 상단
  // 설명 참고) 그 forColor로 해석하면 된다.
  //
  // 다만 지금 노드 자체는 아직 한 번도 분석된 적이 없을 수 있다(막 이동한 직후 아직
  // kata-analyze의 첫 결과가 안 왔거나, 애초에 분석 없이 지나온 위치) - 이 경우
  // 그대로 50:50으로 보여주면 착수/이동할 때마다 잠깐 반반으로 꺼졌다가 몇 백ms 뒤
  // 값이 돌아오는 것처럼 보인다. 그래서 현재 노드부터 시작해 조상 방향으로(자기 자신
  // 포함, boardStore.ancestorChain이 가까운 순서로 줌) 캐싱된 결과가 있는 가장 가까운
  // 노드의 값을 대신 보여준다 - 완전히 같은 위치는 아닐 수 있지만 보통 한두 수 차이라
  // 훨씬 자연스럽고, 그마저도 하나 없으면(트리 전체가 한 번도 분석된 적 없음) 그제서야
  // 50:50으로 표시한다.
  import { analysisStore } from "../../stores/analysis.svelte";
  import { boardStore } from "../../stores/board.svelte";

  const blackWinrate = $derived.by(() => {
    for (const nodeId of boardStore.ancestorChain) {
      const cached = analysisStore.forNode(nodeId);
      const top = cached?.result.candidates[0];
      if (!top) continue;
      return cached.forColor === "black" ? top.winrate : 1 - top.winrate;
    }
    return 0.5;
  });

  const blackPercent = $derived(blackWinrate * 100);
  const whitePercent = $derived(100 - blackPercent);

  // scoreLead도 winrate와 같은 기준(cached.forColor)으로 온다 - blackWinrate와 같은
  // 방식으로 "흑 기준"(양수 = 흑이 유리)으로 정규화해 조상 체인을 따라간다. 막대
  // 자체는 계속 winrate 비율로 그리고(직관적인 흑/백 우세 시각화), 하단 글씨만
  // scoreLead로 바꿔 "몇 집 차이인지"라는 더 구체적인 정보를 보여준다.
  const blackScoreLead = $derived.by(() => {
    for (const nodeId of boardStore.ancestorChain) {
      const cached = analysisStore.forNode(nodeId);
      const top = cached?.result.candidates[0];
      if (!top) continue;
      return cached.forColor === "black" ? top.scoreLead : -top.scoreLead;
    }
    return 0;
  });

  // 지금 둘 차례인 돌(boardStore.currentTurn) 기준 집 차이. blackScoreLead는 항상
  // "흑 기준"으로 정규화돼 있으므로, 지금이 백 차례면 부호만 뒤집으면 됨.
  const currentTurnScoreLead = $derived(
    boardStore.currentTurn === "black" ? blackScoreLead : -blackScoreLead,
  );
  const scoreLeadLabel = $derived(
    currentTurnScoreLead >= 0
      ? `+${currentTurnScoreLead.toFixed(1)}`
      : currentTurnScoreLead.toFixed(1),
  );
</script>

<div class="winrate-graph">
  <div
    class="winrate-bar"
    title="Black {blackPercent.toFixed(1)}% / White {whitePercent.toFixed(1)}%"
  >
    <div class="segment white" style="flex-basis: {whitePercent}%"></div>
    <div class="segment black" style="flex-basis: {blackPercent}%"></div>
  </div>
  <span class="winrate-label">{scoreLeadLabel}</span>
</div>

<style>
  .winrate-graph {
    flex: 0 0 auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }

  .winrate-label {
    font-variant-numeric: tabular-nums;
    font-size: 13px;
    font-weight: bold;
    line-height: 1;
    opacity: 0.85;
  }

  .winrate-bar {
    display: flex;
    overflow: hidden;
    border: 1px solid #444;
    border-radius: 6px;
  }

  .segment {
    flex-grow: 0;
    flex-shrink: 0;
    /* 승률이 갱신될 때 막대 비율이 뚝 끊기지 않고 부드럽게 움직이도록 */
    transition: flex-basis 0.3s ease;
  }

  .segment.white {
    background: #f2f2f2;
  }

  .segment.black {
    background: #111111;
  }

  /* 세로 모드: 바둑판 바로 아래 가로 막대. DOM 순서(흰 -> 검)대로면 왼쪽이 흰색,
     오른쪽이 검은색이 되어 "흑은 오른쪽" 요구사항을 그대로 만족함. 폭을 100%가 아니라
     100cqmin(=GoBoard와 동일한 기준)으로 둬서 바둑판 정사각형의 실제 폭과 정확히
     같아지게 함 - board-area에 남는 여백이 있어도 그 여백만큼 막대만 더 넓어지는
     일이 없어 바둑판 가장자리에 그대로 맞닿는다. */
  @media (orientation: portrait) {
    .winrate-graph {
      width: 100cqmin;
    }

    .winrate-bar {
      flex-direction: row;
      width: 100%;
      height: 28px;
    }

    .segment {
      height: 100%;
    }
  }

  /* 가로 모드/데스크탑: 바둑판 바로 오른쪽에 붙는 얇은 세로 막대. board-area가
     가로(row)로 바뀌므로 GoBoard와 나란히 놓인다. 높이도 폭과 같은 이유로
     100%(board-area 전체 높이) 대신 100cqmin(=GoBoard의 실제 한 변)으로 둬서 막대
     상/하단이 바둑판 상/하단과 정확히 맞아떨어지게 함. column-reverse로 DOM 순서
     (흰 -> 검)를 뒤집어 검은색이 위쪽에 오도록 함. */
  @media (orientation: landscape) {
    .winrate-bar {
      flex-direction: column-reverse;
      flex: 0 0 auto;
      width: 28px;
      height: 100cqmin;
    }

    .segment {
      width: 100%;
    }
  }
</style>
