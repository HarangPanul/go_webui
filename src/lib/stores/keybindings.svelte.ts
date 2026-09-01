// 키보드 단축키 설정: Settings에서 사용자가 원하는 키로 재배정할 수 있는 액션들.
// 방향키(임시 선택 이동)는 구조상 고정이며 여기서 다루지 않음.
// localStorage에 저장해 앱을 다시 켜도 유지됨.

export type KeyAction = "confirmMove" | "changeColor" | "back" | "removeLastMove";

const STORAGE_KEY = "go-webui.keybindings";

const DEFAULT_BINDINGS: Record<KeyAction, string> = {
  confirmMove: " ",
  changeColor: "c",
  back: "b",
  removeLastMove: "r",
};

// KeyboardEvent.key 값을 비교 가능한 형태로 정규화. 알파벳 한 글자는 대소문자를
// (Shift 여부와 무관하게) 같은 키로 취급하고, "Enter"/"Backspace" 같은 특수 키
// 이름은 그대로 둠.
function normalize(key: string): string {
  return key.length === 1 ? key.toLowerCase() : key;
}

function loadBindings(): Record<KeyAction, string> {
  if (typeof localStorage === "undefined") return { ...DEFAULT_BINDINGS };
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULT_BINDINGS };
    const parsed = JSON.parse(raw) as Partial<Record<KeyAction, string>>;
    // 저장된 값에 없는 액션(예: 이후 새 액션 추가)은 기본값으로 채움
    return { ...DEFAULT_BINDINGS, ...parsed };
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
    // 그 액션의 키는 비워짐.
    setKey(action: KeyAction, key: string) {
      const norm = normalize(key);
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
    // 키보드 이벤트의 key가 해당 액션에 등록된 키와 일치하는지 확인 (대소문자 무시)
    matches(action: KeyAction, evtKey: string): boolean {
      const bound = bindings[action];
      if (!bound) return false;
      return normalize(bound) === normalize(evtKey);
    },
  };
}

export const keybindingsStore = createKeybindingsStore();
