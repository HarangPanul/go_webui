// 컨테이너 안의 포커스 가능한 요소(버튼/입력/select 등) 사이를 위/아래 화살표로
// 옮겨 다니게 하는 범용 헬퍼. 특정 컴포넌트를 하드코딩해서 등록하는 방식이 아니라
// 키를 누르는 시점마다 DOM을 그대로 querySelectorAll로 다시 훑으므로, 나중에 그
// 컨테이너 안에 새 버튼/입력을 추가해도 별도 등록 없이 자동으로 이 탐색 대상에
// 포함된다 - Settings 화면처럼 계속 항목이 늘어나는 화면에 적합.
//
// <input type="number">는 위/아래 화살표가 브라우저 기본 동작으로 값을 증가/
// 감소시키는데(스피너), 이 헬퍼를 쓰면 그 대신 항상 포커스 이동으로 동작한다(아래
// preventDefault). 반면 <textarea>(여러 줄 입력이라 위/아래로 줄 사이를 이동해야
// 함)와 <select>(위/아래로 옵션 자체를 바꾸는 것도 기본 동작)는 원래 동작을 그대로
// 두기 위해 이 탐색에서 제외한다.
const FOCUSABLE_SELECTOR =
  'button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

export function handleRovingArrowKeys(evt: KeyboardEvent, container: HTMLElement): void {
  if (evt.key !== "ArrowUp" && evt.key !== "ArrowDown") return;

  const target = evt.target as HTMLElement | null;
  if (target && (target.tagName === "TEXTAREA" || target.tagName === "SELECT")) return;

  const focusables = Array.from(
    container.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR),
  ).filter((el) => el.offsetParent !== null); // 접힌 CollapsibleSection 등 화면에 안 보이는 요소는 제외

  if (focusables.length === 0) return;

  // 지금 포커스가 이 컨테이너 안 어디에도 없으면(예: 화면이 막 열려 아무것도
  // 포커스되지 않은 상태) -1로 취급 - 그러면 아래 계산에서 ArrowDown은 첫 번째
  // 요소로, ArrowUp은 마지막 요소로 자연스럽게 이동한다.
  const currentIndex = target ? focusables.indexOf(target) : -1;
  const delta = evt.key === "ArrowDown" ? 1 : -1;
  const nextIndex = ((currentIndex + delta) % focusables.length + focusables.length) % focusables.length;

  evt.preventDefault();
  focusables[nextIndex]?.focus();
}
