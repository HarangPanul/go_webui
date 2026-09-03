// kata-analyze 스트리밍 결과 store. Tauri event("kata-analyze")를 구독해 갱신.
//
// 게임 트리의 노드마다 결과를 따로 캐싱한다(Map<nodeId, NodeAnalysis>) - 예전에는
// "최신 결과 하나 + 그 결과가 어느 색 기준인지(forColor)"를 따로 들고 있었는데, 착수
// 직후 kata-analyze가 새 위치 기준으로 재시작되면 forColor는 곧바로(동기적으로) 새
// 색으로 바뀌는 반면 실제 결과(latest)는 새 스트림의 첫 "info" 줄이 도착할 때까지
// (수백ms~) 옛 위치의 데이터를 그대로 들고 있었다. 그 사이 짧은 순간, 옛 위치에서
// 계산된 값을 이미 바뀐 forColor 기준으로 잘못 재해석해버려서 승률/ownership이 착수할
// 때마다 잠깐 반대로 튀었다가 새 데이터가 오면 되돌아오는(요동치는) 버그가 있었다.
//
// 이제는 결과와 그 결과가 어느 노드/색 기준인지를 항상 같은 이벤트로 함께 받아
// (KataAnalyzeEvent - Rust가 kata-analyze 명령을 보내는 바로 그 시점에 노드/색을
// 기록해두므로 둘이 어긋날 일이 없다) 노드 id를 키로 캐싱하므로, 화면에 표시할 때도
// 항상 "같은 노드에 대해 함께 도착한 값" 한 쌍만 보여주게 된다. 부수 효과로 두 가지가
// 함께 해결된다:
// - 노드마다 결과가 남아있으므로 뒤로/앞으로 이동해도 그 노드에서 마지막으로 받은
//   분석이 그대로 남아 있다(다시 분석하지 않아도).
// - 분석을 끄거나(toggleAnalysis) 일시적으로 스트림이 끊겨도 마지막 결과가 사라지지
//   않으므로 winrate bar가 5:5로 되돌아가지 않는다 - 새 게임을 시작하는 등 명시적으로
//   reset()을 호출할 때만 비워진다.
import { listen } from "@tauri-apps/api/event";
import type { KataAnalyzeEvent, KataAnalyzeResult } from "../generated/bindings";
import { gameTreeStore } from "./gameTree.svelte";

interface NodeAnalysis {
  result: KataAnalyzeResult;
  forColor: "black" | "white";
}

function createAnalysisStore() {
  // 노드 id -> 그 노드에서 마지막으로 받은 kata-analyze 결과. .set()으로 직접
  // mutate하지 않고 매번 새 Map으로 교체(reassign)해야 $state가 반응함(Map/Set은
  // $state가 자동으로 deep-proxy하는 대상이 아니라 일반 객체/배열만 해당).
  let results = $state<Map<number, NodeAnalysis>>(new Map());
  // AnalysisOverlay(지점별 bluespot)/OwnershipOverlay 각각의 표시 여부. kata-analyze
  // 자체는(요청할 때 "ownership true"를 항상 붙여서) 두 종류 데이터를 늘 함께
  // 스트리밍해오지만, 그중 무엇을 바둑판에 겹쳐 그릴지는 이 두 토글이 독립적으로
  // 결정한다 - 하나만 켜져 있어도 스트림 자체는 계속 돌아야 하고(GameControls의
  // streamWanted), 꺼진 쪽은 데이터를 계속 받아 캐싱만 해두었다가 나중에 켜지는
  // 순간 즉시 보여줄 수 있어야 하므로 "받는지"와 "그리는지"를 분리해야 한다.
  let showAnalysis = $state(false);
  let showOwnership = $state(false);

  listen<KataAnalyzeEvent>("kata-analyze", (event) => {
    const { nodeId, forColor, ...result } = event.payload;
    const next = new Map(results);
    next.set(nodeId, { result: result as KataAnalyzeResult, forColor });
    results = next;
  });

  return {
    // 지금 보드가 위치한 노드의 분석 결과(없으면 null - 그 노드를 아직 한 번도
    // 분석한 적이 없다는 뜻). WinrateGraph/AnalysisOverlay/OwnershipOverlay가
    // 이걸로 result/forColor를 항상 한 쌍으로만 읽는다.
    get current() {
      return results.get(gameTreeStore.nodeId) ?? null;
    },
    // 임의의 노드 id로 조회. WinrateGraph가 gameTreeStore.ancestorChain을 따라가며
    // 가장 가까운(자기 자신 포함) 조상 중 캐싱된 결과가 있는 노드를 찾는 데 사용.
    forNode(nodeId: number) {
      return results.get(nodeId) ?? null;
    },
    get showAnalysis() {
      return showAnalysis;
    },
    toggleAnalysis() {
      showAnalysis = !showAnalysis;
    },
    get showOwnership() {
      return showOwnership;
    },
    toggleOwnership() {
      showOwnership = !showOwnership;
    },
    reset() {
      results = new Map();
      showAnalysis = false;
      showOwnership = false;
    },
  };
}

export const analysisStore = createAnalysisStore();
