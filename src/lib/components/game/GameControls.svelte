<script lang="ts">
  // 버튼 패널: [착수 확정]/[취소] 전환, 다음 착수 색 전환, 대국 모드 전환 등 (설정 진입은 TopBar로 이동)
  // portrait에서는 하단 가로 바, landscape/desktop에서는 우측 세로 컬럼(GameScreen 레이아웃이 방향 전환)
  import { t } from "../../i18n";
  import { gameTreeStore } from "../../stores/gameTree.svelte";
  import { pendingMoveStore } from "../../stores/pendingMove.svelte";
  import { engineAssignmentStore } from "../../stores/engineAssignment.svelte";
  import { serverProfilesStore } from "../../stores/serverProfiles.svelte";
  import { gtpStore } from "../../stores/gtp.svelte";
  import { connectionStore } from "../../stores/connection.svelte";
  import { analysisStore } from "../../stores/analysis.svelte";
  import { analysisEngineStore } from "../../stores/analysisEngine.svelte";
  import { holdToRepeat } from "../../utils/holdToRepeat";
  import Button from "../ui/Button.svelte";
  import Select from "../ui/Select.svelte";
  import stoneBlackUrl from "../../../assets/stone-black.png";
  import stoneWhiteUrl from "../../../assets/stone-white.png";

  const stoneImages = { black: stoneBlackUrl, white: stoneWhiteUrl };

  // 뒤로/앞으로/마지막 수 제거 버튼을 꾹 누르면 0.5초 뒤부터 빠르게 반복 실행됨
  // (holdToRepeat.ts 참고) - 컴포넌트 최상단에서 한 번만 만들어 재사용해야 반복 도중
  // 리렌더가 일어나도 내부 타이머 상태가 끊기지 않는다.
  const backHold = holdToRepeat(
    () => gameTreeStore.goBack(),
    () => gameTreeStore.canGoBack,
  );
  const forwardHold = holdToRepeat(
    () => gameTreeStore.goForward(),
    () => gameTreeStore.canGoForward,
  );
  const removeLastMoveHold = holdToRepeat(
    () => gameTreeStore.removeLastMove(),
    () => gameTreeStore.canGoBack,
  );

  // 흑/백에 배정할 수 있는 후보는 "지금 연결되어 있는 프로필"뿐 - 연결 안 된
  // 프로필을 배정해봐야 실제로 둘 세션이 없어 아무 동작도 하지 않는다. 같은 프로필을
  // 흑/백 둘 다에 배정하면 엔진이 자기 자신과 대국하듯 계속 두고, 둘 다 배정하지
  // 않으면(드롭다운을 "사람"으로 두면) 자동 착수 없이 사람이 양쪽을 다 둠. 실제
  // 자동 착수(genmove) 로직은 confirmMove() 이후 백엔드(commands::game::confirm_move)가
  // 처리하고, 여기서는 그 배정값만 바꿔 전송한다.
  const connectedProfiles = $derived(
    serverProfilesStore.profiles.filter((p) =>
      connectionStore.connectedProfileIds.includes(p.id),
    ),
  );

  function handleAssignmentChange(color: "black" | "white", event: Event) {
    const value = (event.currentTarget as HTMLSelectElement).value;
    engineAssignmentStore.setAssignment(color, value === "" ? null : value);
  }

  // Analysis(엔진 분석 미리 받아오기) 버튼: 켜지면 "start_kata_analyze" 전용 커맨드로
  // 연결된 세션에 "kata-analyze <interval> ownership true"를 보낸다 - 이 명령은 다른
  // 입력이 들어올 때까지 계속 "info ..." 줄을 스트리밍하는 특수한 GTP 명령이라,
  // 백엔드가 그 줄들을 즉시 파싱해 "kata-analyze" 이벤트로 흘려보내고(analysisStore가
  // 이미 구독 중), 이 invoke() 자체의 Promise는 분석이 멈출 때까지 계속 pending
  // 상태로 남아있으므로 await하지 않는다(= fire-and-forget).
  //
  // send_gtp_command 대신 이 전용 커맨드를 쓰는 이유: 백엔드가 명령을 실제로 보내는
  // 바로 그 시점에 "지금 게임 트리 어느 노드, 어느 색 차례인지"를 함께 기록해둬야
  // (commands::gtp::start_kata_analyze / process.rs::AnalysisContext) 이어서 오는
  // "info" 결과들이 정확히 그 노드에 태깅되어 emit된다. forColor를 프런트에서 별도로
  // 추적해 끼워 맞추면, 착수 직후 forColor는 곧바로 바뀌는데 실제 데이터는 나중에
  // 도착하는 타이밍 차이 때문에 승률/ownership이 매 착수마다 잠깐 반대로 튀는
  // (요동치는) 결과를 낳는다.
  //
  // "ownership true"는 항상 붙여 보낸다 - kata-analyze는 Analysis(블루스팟)/Ownership
  // 두 기능이 공유하는 단 하나의 스트림이라, 굳이 옵션을 나눠서 필요할 때만 요청할
  // 이유가 없다(끄고 켤 때마다 옵션이 다르면 스트림을 재시작해야 해서 그사이 잠깐
  // 분석이 끊기고 다시 워밍업해야 함).
  const ANALYSIS_INTERVAL_CENTISECONDS = 50; // 0.5초 간격으로 info 업데이트

  function startAnalysis() {
    gtpStore.startKataAnalyze(ANALYSIS_INTERVAL_CENTISECONDS).catch(() => {
      if (analysisStore.showAnalysis) analysisStore.toggleAnalysis();
    });
  }

  // Analysis(블루스팟)와 Ownership은 "같은 kata-analyze 스트림을 공유한다"는 점만
  // 같을 뿐, 그 결과 중 무엇을 화면에 그릴지는 analysisStore의 두 토글
  // (showAnalysis/showOwnership)이 각각 독립적으로 결정한다 - 스트림 자체는 둘 중
  // 하나라도 켜져 있으면 계속 돌면서 두 종류 데이터를 항상 함께 캐싱해두므로, 꺼둔
  // 쪽을 나중에 켜도 그 즉시 이미 받아둔 최신 결과를 보여줄 수 있다(AnalysisOverlay/
  // OwnershipOverlay가 각자 자기 토글만 보고 그릴지 말지 정함). 요청사항대로 어느
  // 한쪽을 껐다고 다른 한쪽까지 같이 꺼지면 안 되므로, 실제 스트림 on/off는 이 둘의
  // OR로만 판단한다.
  const streamWanted = $derived(analysisStore.showAnalysis || analysisStore.showOwnership);

  function toggleAnalysis() {
    if (connectionStore.connectedProfileIds.length === 0) return;
    analysisStore.toggleAnalysis();
  }

  function toggleOwnership() {
    if (connectionStore.connectedProfileIds.length === 0) return;
    analysisStore.toggleOwnership();
  }

  // Analysis/Ownership에 쓸 세션을 자동(지금 차례 색에 배정된 세션 -> 없으면 연결된
  // 아무 세션) 대신 직접 고른다 - self-play처럼 흑/백 둘 다 같은 서버에 배정해
  // genmove가 끊임없이 도는 상황에서는 그 서버가 kata-analyze도 함께 맡으면 매
  // genmove가 곧바로 분석 스트림을 인터럽트해버려 결과가 사실상 전혀 안 들어온다
  // (GTP 프로토콜 자체의 제약 - 한 세션이 genmove/kata-analyze를 동시에 못 함).
  // 다른 서버를 하나 더 연결해 여기서 지정해두면 서로 다른 세션을 쓰게 되어 이 문제가
  // 없다. 지금 스트림이 이미 켜져 있으면(streamWanted) 바로 이어서 startAnalysis()를
  // 다시 호출해 새로 고른 세션으로 즉시 전환한다 - 안 그러면 다음 착수/이동이 있을
  // 때까지는 옛 세션으로 계속 분석 중인 것처럼 보인다.
  function handleAnalysisEngineChange(event: Event) {
    const value = (event.currentTarget as HTMLSelectElement).value;
    analysisEngineStore.setProfileId(value === "" ? null : value).then(() => {
      if (streamWanted) startAnalysis();
    });
  }

  // streamWanted가 꺼지는 쪽으로 바뀔 때(=Analysis/Ownership 둘 다 꺼졌을 때)만 실제로
  // 인터럽트를 보내고, 켜지는 쪽으로 바뀌거나(둘 중 하나라도 켜짐) 켜진 채로 보드
  // 위치가 바뀌면(사람 착수/엔진 자동 응수를 엔진에 미러링하는 "play"/"genmove" 명령도
  // 진행 중이던 kata-analyze 스트리밍을 함께 멈추게 하므로) 새 위치 기준으로 다시
  // 시작한다. wasStreaming은 반응형 상태가 아니라 "직전에 실제로 스트림을 켜둔 적
  // 있는지" 기억만 하는 일반 변수 - 꺼진 상태에서 보드 위치만 바뀔 때(분석 자체를 아무도
  // 원하지 않는 평상시 대국) 매 수마다 쓸데없이 인터럽트 명령을 보내지 않기 위함.
  let wasStreaming = false;
  $effect(() => {
    gameTreeStore.lastMove;
    if (streamWanted) {
      wasStreaming = true;
      startAnalysis();
    } else if (wasStreaming) {
      wasStreaming = false;
      // kata-analyze는 "다른 입력"이 들어와야 멈추는 스트리밍 명령이라, 아무 GTP
      // 명령이나 하나 보내면 그 시점에 분석이 멈춘다. 결과 자체는 필요 없으므로
      // fire-and-forget. analysisStore는 여기서 비우지 않는다 - 각 노드별로 마지막에
      // 받은 결과를 그대로 캐싱해두는 게 이 store의 설계이므로(analysis.svelte.ts
      // 참고), 여기서 reset()을 부르면 WinrateGraph가 그 즉시 5:5로 되돌아가버린다.
      // "분석을 멈춤"은 "지금까지 알아낸 결과를 지움"이 아니라 "더 이상 새로 갱신하지
      // 않음"이어야 하므로 캐시는 그대로 두고 갱신만 멈춘다.
      gtpStore.stopKataAnalyze().catch(() => {});
    }
  });

  // 연결이 하나도 안 남으면 더 이상 분석을 이어갈 수 없으므로 토글 표시도 꺼두고,
  // 남아있던 결과도 비워 WinrateGraph/AnalysisOverlay/OwnershipOverlay가 끊긴 연결의
  // 옛 결과를 계속 보여주지 않게 한다(analysisStore.reset()이 showAnalysis/
  // showOwnership을 모두 함께 꺼줌).
  $effect(() => {
    if (connectionStore.connectedProfileIds.length === 0) {
      analysisStore.reset();
    }
  });
</script>

<div class="game-controls">
  <Button
    shape="circle"
    class="color-toggle"
    title={t("game.switchColor")}
    aria-label={t("game.switchColor")}
    onclick={() => gameTreeStore.toggleTurn()}
  >
    <img src={stoneImages[gameTreeStore.currentTurn]} alt={gameTreeStore.currentTurn} />
  </Button>
  <!-- 흑/백에 배정할 엔진을 고르는 드롭다운. 지금 연결된 프로필만 후보로 뜨고,
  "사람"을 고르면 그 색은 자동 착수 없이 사람이 둠(engineAssignment.svelte.ts 참고). -->
  <label class="engine-assign">
    <span>{t("game.engineColor.black")}</span>
    <Select
      value={engineAssignmentStore.assignment.black ?? ""}
      onchange={(e) => handleAssignmentChange("black", e)}
    >
      <option value="">{t("game.engineColor.human")}</option>
      {#each connectedProfiles as profile (profile.id)}
        <option value={profile.id}>{profile.name || profile.host}</option>
      {/each}
    </Select>
  </label>
  <label class="engine-assign">
    <span>{t("game.engineColor.white")}</span>
    <Select
      value={engineAssignmentStore.assignment.white ?? ""}
      onchange={(e) => handleAssignmentChange("white", e)}
    >
      <option value="">{t("game.engineColor.human")}</option>
      {#each connectedProfiles as profile (profile.id)}
        <option value={profile.id}>{profile.name || profile.host}</option>
      {/each}
    </Select>
  </label>
  <Button
    active={analysisStore.showAnalysis}
    title={t("game.analysis")}
    aria-pressed={analysisStore.showAnalysis}
    disabled={connectionStore.connectedProfileIds.length === 0}
    onclick={toggleAnalysis}
  >
    {t("game.analysis")}
  </Button>
  <!-- Analysis 버튼과는 독립적으로 켜고 끌 수 있음 - 이 버튼이 켜지면 Analysis가
  꺼져 있어도 streamWanted가 켜져서 kata-analyze 스트림이 알아서 시작된다(위 $effect
  참고). 연결만 되어 있으면 되므로 disabled 조건은 Analysis 버튼과 동일. -->
  <Button
    active={analysisStore.showOwnership}
    title={t("game.ownership")}
    aria-pressed={analysisStore.showOwnership}
    disabled={connectionStore.connectedProfileIds.length === 0}
    onclick={toggleOwnership}
  >
    {t("game.ownership")}
  </Button>
  <!-- 자동(지금 차례 색에 배정된 세션 -> 없으면 연결된 아무 세션) 대신 Analysis/
  Ownership에 쓸 세션을 직접 고름 - self-play로 흑/백 둘 다 같은 서버에 배정해뒀을 때
  분석이 전혀 안 들어오는 문제의 우회책(handleAnalysisEngineChange 참고). 연결된
  프로필이 하나뿐이면 골라봐야 의미가 없으므로 둘 이상 연결됐을 때만 보여준다. -->
  {#if connectedProfiles.length > 1}
    <label class="engine-assign">
      <span>{t("game.analysisEngine")}</span>
      <Select value={analysisEngineStore.profileId ?? ""} onchange={handleAnalysisEngineChange}>
        <option value="">{t("game.analysisEngineAuto")}</option>
        {#each connectedProfiles as profile (profile.id)}
          <option value={profile.id}>{profile.name || profile.host}</option>
        {/each}
      </Select>
    </label>
  {/if}
  <!-- 착수 없이 차례만 넘김. pendingMove 여부와 무관하게 항상 누를 수 있음(누르면
  진행 중이던 임시 선택은 알아서 비워짐 - gameTree.svelte.ts::passMove 참고). -->
  <Button onclick={() => gameTreeStore.passMove()}>
    {t("game.pass")}
  </Button>
  <!-- 게임 트리에서 부모/자식 노드로 이동. 텍스트 대신 화살표로 표시해 키보드 없이도
  직관적으로 누를 수 있게 함 - 실제 동작(단축키 "["/"]" 포함)은 gameTreeStore.goBack()/
  goForward()와 동일. 자식 쪽은 여러 갈래가 있어도 가장 마지막으로 방문했던 자식으로
  이동함(game::GameTree::go_forward 참고). -->
  <Button
    title={t("game.back")}
    aria-label={t("game.back")}
    disabled={!gameTreeStore.canGoBack}
    onclick={() => gameTreeStore.goBack()}
    {...backHold}
  >
    ←
  </Button>
  <Button
    title={t("game.goForward")}
    aria-label={t("game.goForward")}
    disabled={!gameTreeStore.canGoForward}
    onclick={() => gameTreeStore.goForward()}
    {...forwardHold}
  >
    →
  </Button>
  <!-- 이미 돌이 있는 칸을 가리키고 있을 때도 버튼 자체는 항상 그대로 표시하되,
  confirmMove()가 그 경우 아무 동작도 하지 않으므로 눌러도 아무 효과가 없음. 취소는
  별도 버튼 없이 BoardCanvas에서 현재 임시 선택 지점을 다시 누르면 됨(cancelPending
  호출) - board/BoardCanvas.svelte 참고. -->
  <Button
    variant="primary"
    disabled={!pendingMoveStore.pendingMove}
    onclick={() => gameTreeStore.confirmMove()}
  >
    {t("game.confirmMove")}
  </Button>
  <Button
    variant="danger"
    disabled={!gameTreeStore.canGoBack}
    onclick={() => gameTreeStore.removeLastMove()}
    {...removeLastMoveHold}
  >
    {t("game.removeLastMove")}
  </Button>
</div>

<style>
  .game-controls {
    display: flex;
    flex-direction: row;
    flex-wrap: wrap;
    gap: var(--space-4);
    width: 100%;
  }

  /* 버튼 자체의 색/테두리/비활성 스타일은 이제 ui/Button.svelte가 담당 - 여기서는
     이 버튼 패널 특유의 배치(늘어나서 줄바꿈 + 최소 너비)만 지정. 원형 아이콘
     버튼(color-toggle)은 Button.svelte의 shape="circle"이 이미 늘어나지 않게
     처리하므로 여기서 제외. */
  .game-controls :global(.ui-button:not(.circle)) {
    flex: 1 1 auto;
    min-width: 96px;
    padding: var(--space-5) var(--space-6);
  }

  /* 다음 착수 색 전환 버튼 안의 돌 이미지 */
  .game-controls :global(.color-toggle img) {
    width: 100%;
    height: 100%;
    object-fit: contain;
    pointer-events: none;
  }

  /* 흑/백 엔진 배정 드롭다운 - 다른 버튼들과 같은 flex 아이템으로 나란히 줄바꿈되되,
     레이블+select를 세로로 쌓아 라벨이 잘리지 않게 함. */
  .engine-assign {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    flex: 1 1 auto;
    min-width: 96px;
    font-size: 0.85rem;
  }

  .engine-assign :global(.ui-input) {
    width: 100%;
  }
</style>
