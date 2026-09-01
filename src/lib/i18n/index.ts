// 최소 i18n 유틸: 영어/한국어만 지원 (확장 가능한 구조)
import en from "./en.json";
import ko from "./ko.json";

const dictionaries = { en, ko } as const;
export type Locale = keyof typeof dictionaries;

let currentLocale: Locale = "en";

export function setLocale(locale: Locale) {
  currentLocale = locale;
}

export function t(key: keyof typeof en): string {
  return dictionaries[currentLocale][key] ?? key;
}
