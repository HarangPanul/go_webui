// UI 언어 선택 상태. i18n/index.ts의 t()가 이 store의 현재 값을 읽어 사전을
// 고르므로, setLocale()을 부르면 이미 화면에 떠 있는 모든 t(...) 호출도 즉시
// 반응형으로 다시 렌더링된다(i18n/index.ts는 평범한 .ts라 룬을 쓸 수 없어 실제
// $state는 여기 있어야 함). instantMove.svelte.ts와 같은 패턴으로 localStorage에
// 저장해 앱을 다시 켜도 유지됨.
import type { Locale } from "../i18n";

const STORAGE_KEY = "go-webui.locale";

function isLocale(value: string): value is Locale {
  return value === "en" || value === "ko";
}

function loadLocale(): Locale {
  if (typeof localStorage === "undefined") return "en";
  const stored = localStorage.getItem(STORAGE_KEY);
  return stored && isLocale(stored) ? stored : "en";
}

function createLocaleStore() {
  let locale = $state(loadLocale());

  return {
    get current() {
      return locale;
    },
    set(value: Locale) {
      locale = value;
      if (typeof localStorage !== "undefined") {
        localStorage.setItem(STORAGE_KEY, value);
      }
    },
  };
}

export const localeStore = createLocaleStore();
