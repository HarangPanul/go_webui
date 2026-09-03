<script lang="ts">
  // UI 언어 선택. setLocale()이 stores/locale.svelte.ts의 $state를 갱신하므로
  // 고르는 즉시 이 화면을 포함해 이미 떠 있는 모든 t(...) 호출이 반응형으로 다시
  // 렌더링된다(i18n/index.ts 참고). instantMoveStore와 동일하게 localStorage에
  // 저장돼 앱을 다시 켜도 유지됨.
  //
  // 언어 이름 자체는 그 언어로 표시(번역하지 않음) - 지금 UI 언어가 뭐든 자기
  // 모국어 이름은 항상 알아볼 수 있어야 하기 때문(다른 다국어 앱들의 일반적인 관례).
  import { t, setLocale, type Locale } from "../../i18n";
  import { localeStore } from "../../stores/locale.svelte";
  import FormField from "../ui/FormField.svelte";
  import Select from "../ui/Select.svelte";

  const OPTIONS: { value: Locale; label: string }[] = [
    { value: "en", label: "English" },
    { value: "ko", label: "한국어" },
  ];

  function handleChange(event: Event) {
    setLocale((event.target as HTMLSelectElement).value as Locale);
  }
</script>

<FormField label={t("settings.language")} layout="row">
  <Select value={localeStore.current} onchange={handleChange}>
    {#each OPTIONS as { value, label } (value)}
      <option {value}>{label}</option>
    {/each}
  </Select>
</FormField>
