// 버튼을 짧게 클릭하면 onclick 한 번만 실행되지만, 꾹 누르고 있으면 HOLD_DELAY_MS 뒤부터
// REPEAT_INTERVAL_MS 간격으로 같은 동작을 계속 반복 실행하게 해주는 헬퍼 - 뒤로/앞으로
// 가기나 마지막 수 제거처럼 "여러 번 누르면 여러 수를 넘기는" 버튼에서 매번 클릭하지
// 않고도 꾹 눌러 빠르게 여러 수를 넘길 수 있게 하기 위함. Svelte 5는 onpointerdown 등
// 이벤트 props를 그냥 프로퍼티로 받으므로 별도 action 없이 Button에 그대로 스프레드해서
// 쓴다. onclick은 여기서 다루지 않음 - 짧게 눌렀다 뗄 때의 "한 번"은 브라우저 기본
// click 이벤트에 그대로 맡긴다.
//
// action이 Promise를 반환하면(goBack/removeLastMove 등은 invoke() 호출이라 비동기)
// 그 완료를 기다린 뒤에야 다음 반복을 예약한다 - setInterval로 무작정 겹쳐 부르면
// 백엔드 응답이 늦게 도착했을 때 스냅샷이 뒤섞여 반영될 수 있기 때문(마지막에 도착한
// 응답이 이기는 게 아니라, 순서 보장 없이 덮어써질 수 있음).
//
// 호출하는 쪽(GameControls.svelte)에서 이 함수는 컴포넌트 스크립트 최상단에서 딱 한
// 번만 호출해 반환값을 재사용해야 한다 - 매 렌더마다 새로 호출하면 delayTimer/
// cancelled를 담은 클로저도 매번 새로 생겨서, 반복 도중 상태 변화로 리렌더가 일어나면
// (goBack() 자체가 스토어를 갱신해 리렌더를 유발함) 그 다음 pointerup이 방금 시작된
// 반복이 아니라 엉뚱한(새) 클로저의 타이머를 멈추려 들어 반복이 멈추지 않게 된다.
const HOLD_DELAY_MS = 500;
const REPEAT_INTERVAL_MS = 120;

export function holdToRepeat(action: () => unknown, canContinue: () => boolean) {
  let delayTimer: ReturnType<typeof setTimeout> | undefined;
  let cancelled = true;

  function stop() {
    clearTimeout(delayTimer);
    delayTimer = undefined;
    cancelled = true;
  }

  async function tick() {
    if (cancelled || !canContinue()) return;
    await action();
    if (cancelled || !canContinue()) return;
    delayTimer = setTimeout(tick, REPEAT_INTERVAL_MS);
  }

  function start(event: PointerEvent) {
    if (event.button !== 0) return; // 왼쪽 버튼/터치/펜만 - 우클릭 등은 무시
    stop();
    cancelled = false;
    delayTimer = setTimeout(tick, HOLD_DELAY_MS);
  }

  return {
    onpointerdown: start,
    onpointerup: stop,
    onpointerleave: stop,
    onpointercancel: stop,
  };
}
