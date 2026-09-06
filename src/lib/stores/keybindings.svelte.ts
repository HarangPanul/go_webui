// 키보드 단축키 설정: Settings에서 사용자가 원하는 키로 재배정할 수 있는 액션들.
// 방향키(임시 선택 이동)는 구조상 고정이며 여기서 다루지 않음.
// engineConnect/engineWhite/engineBlack/analysis/ownership은 기본값이 "e" +
// 두 번째 글자로 이뤄진 두 글자 시퀀스(예: "ec")임 - 실제 "e" 다음 키 조합을
// 감지하는 버퍼링은 이 store가 아니라 그걸 실제로 구독하는 쪽
// (KeyboardShortcuts.svelte)이 담당하고, 여기서는 그냥 문자열 하나로 저장/비교만
// 한다(길이가 1이든 2든 matches()는 동일하게 동작).
// localStorage에 저장해 앱을 다시 켜도 유지됨.

export type KeyAction =
  | "confirmMove"
  | "changeColor"
  | "back"
  | "goForward"
  | "removeLastMove"
  | "cancelPendingMove"
  | "engineConnect"
  | "engineWhite"
  | "engineBlack"
  | "openSettings"
  | "analysis"
  | "ownership";

const STORAGE_KEY = "go-webui.keybindings";

const DEFAULT_BINDINGS: Record<KeyAction, string> = {
  confirmMove: " ",
  changeColor: "c",
  // 게임 트리에서 부모/자식 노드로 이동(뒤로/앞으로 가기) - 대괄호는 vim의
  // "[["/"]]" 같은 이동 계열 단축키에서 흔히 쓰이는 조합이라 그 관례를 따름.
  back: "[",
  goForward: "]",
  removeLastMove: "r",
  // Esc는 이미 두 글자 시퀀스 버퍼를 비우는 데 쓰이고 있어서(KeyboardShortcuts.svelte) -
  // 시퀀스 대기 중(예: "e"를 누르고 다음 글자를 기다리는 상태)에 Esc를 누르면 그
  // 버퍼부터 비우는 게 우선이라, 그 순간에는 임시 선택이 있어도 취소되지 않는 사각
  // 지대가 생긴다. 그런 우선순위 다툼이 아예 없는 키를 기본값으로 둔다.
  cancelPendingMove: "Backspace",
  engineConnect: "ec",
  engineWhite: "ew",
  engineBlack: "eb",
  openSettings: "s",
  analysis: "ea",
  ownership: "eo",
};

export const ALL_ACTIONS: KeyAction[] = Object.keys(DEFAULT_BINDINGS) as KeyAction[];

// KeyboardEvent.key 값(또는 두 글자를 이어붙인 시퀀스 버퍼)을 비교 가능한 형태로
// 정규화. Shift 여부와 무관하게 같은 키로 취급하기 위해 전부 소문자로 맞춘다 -
// "Enter"/"ArrowUp" 같은 특수 키 이름도 소문자로 바뀌지만 비교하는 양쪽 모두
// 똑같이 정규화되므로 문제없다.
function normalize(key: string): string {
  return key.toLowerCase();
}

// Esc는 어떤 액션에도 재배정할 수 없다 - 이미 두 글자 시퀀스 버퍼를 비우는 데
// 고정으로 쓰이고 있어서(KeyboardShortcuts.svelte), 다른 액션에도 배정해두면
// 시퀀스 대기 중엔 버퍼 비우기가 항상 먼저 소비해버려 그 액션이 조용히 씹히는
// 사각지대가 생긴다. KeybindingsForm의 "키 변경" 캡처도 Esc를 누르면 취소로
// 처리해 애초에 이 값으로 setKey를 호출하지 않지만, 그 UI를 우회해서 호출되는
// 경로가 생겨도 안전하도록 여기서 한 번 더 막는다.
const RESERVED_KEY = "escape";

// 저장된 값 중 Esc로 배정된 게 있으면 비운다 - setKey가 지금은 막고 있지만, 이
// 제한이 생기기 전에 이미 localStorage에 저장돼 있었을 수 있으므로 불러올 때도
// 한 번 걸러준다.
function sanitize(bindings: Record<KeyAction, string>): Record<KeyAction, string> {
  const result = { ...bindings };
  for (const action of Object.keys(result) as KeyAction[]) {
    if (result[action] && normalize(result[action]) === RESERVED_KEY) {
      result[action] = "";
    }
  }
  return result;
}

function loadBindings(): Record<KeyAction, string> {
  if (typeof localStorage === "undefined") return { ...DEFAULT_BINDINGS };
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULT_BINDINGS };
    const parsed = JSON.parse(raw) as Partial<Record<KeyAction, string>>;
    // 저장된 값에 없는 액션(예: 이후 새 액션 추가)은 기본값으로 채움
    return sanitize({ ...DEFAULT_BINDINGS, ...parsed });
  } catch {
    return { ...DEFAULT_BINDINGS };
  }
}

function createKeybindingsStore() {
  let bindings = $state<Record<KeyAction, string>>(loadBindings());

  function persist() {
    if (typeof localStorage === "undefined") return;
    localStorage.setItem(STORAGE_KEY, JSON.stringify(bindings));
  }

  return {
    get bindings() {
      return bindings;
    },
    keyFor(action: KeyAction): string {
      return bindings[action];
    },
    // action에 새 key를 등록. 다른 액션이 이미 같은 키를 쓰고 있었다면(중복 방지)
    // 그 액션의 키는 비워짐. Esc는 예약된 키라 무시(위 RESERVED_KEY 참고).
    setKey(action: KeyAction, key: string) {
      const norm = normalize(key);
      if (norm === RESERVED_KEY) return;
      const next = { ...bindings };
      for (const other of Object.keys(next) as KeyAction[]) {
        if (other !== action && next[other] && normalize(next[other]) === norm) {
          next[other] = "";
        }
      }
      next[action] = key;
      bindings = next;
      persist();
    },
    resetToDefault(action: KeyAction) {
      bindings = { ...bindings, [action]: DEFAULT_BINDINGS[action] };
      persist();
    },
    // 키보드 이벤트의 key가 해당 액션에 등록된 키와 일치하는지 확인 (대소문자 무시).
    // evtKey 자리에 두 글자를 이어붙인 시퀀스 버퍼("e"+"c" -> "ec")를 넣어도 그대로
    // 동작한다 - 등록된 값과 정규화 후 문자열 전체가 같은지만 비교하기 때문.
    matches(action: KeyAction, evtKey: string): boolean {
      const bound = bindings[action];
      if (!bound) return false;
      return normalize(bound) === normalize(evtKey);
    },
    // key 한 글자가 현재 등록된 두 글자 이상 시퀀스 중 하나의 첫 글자와 일치하는지.
    // KeyboardShortcuts.svelte가 "이 키를 일단 버퍼에 담고 다음 키를 기다려볼
    // 가치가 있는지" 판단하는 데 사용 - 그렇지 않으면 아무 시퀀스와도 무관한 키를
    // 눌렀을 때마다 쓸데없이 타임아웃을 걸게 된다.
    isPrefixKey(key: string): boolean {
      const norm = normalize(key);
      return Object.values(bindings).some((b) => b.length >= 2 && normalize(b)[0] === norm);
    },
  };
}

export const keybindingsStore = createKeybindingsStore();
