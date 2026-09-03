// 최소 i18n 유틸: 영어/한국어만 지원 (확장 가능한 구조)
// 실제 "지금 어느 언어인지" 상태는 stores/locale.svelte.ts가 $state로 들고 있다
// (이 파일은 .svelte.ts가 아니라 룬을 쓸 수 없음) - t()가 매번 그 store를 읽으므로
// setLocale() 호출 즉시 화면에 떠 있는 모든 t(...) 호출이 반응형으로 다시
// 렌더링된다 - LanguageForm.svelte 참고.
import en from "./en.json";
import ko from "./ko.json";
import { localeStore } from "../stores/locale.svelte";

const dictionaries = { en, ko } as const;
export type Locale = keyof typeof dictionaries;

export function setLocale(locale: Locale) {
  localeStore.set(locale);
}

export function t(key: keyof typeof en): string {
  return dictionaries[localeStore.current][key] ?? key;
}
