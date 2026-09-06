<script lang="ts">
  // 로컬 온디바이스 엔진의 genmove 시뮬레이션 수(max_visits) 입력 - KomiForm.svelte와
  // 완전히 같은 패턴. 값이 바뀌면(엔터/포커스 아웃) maxVisitsStore.set()이 저장과
  // 동시에 연결된 엔진에도 바로 반영함(maxVisits.svelte.ts 참고).
  import { t } from "../../i18n";
  import { maxVisitsStore } from "../../stores/maxVisits.svelte";
  import FormField from "../ui/FormField.svelte";
  import NumberInput from "../ui/NumberInput.svelte";

  // 입력 중인 값은 별도 로컬 상태로 두고, maxVisitsStore.value가 바뀔 때(예: 앱 시작
  // 시 백엔드에서 값을 가져온 직후) 그 값으로 다시 채워 넣는다.
  let draft = $state(maxVisitsStore.value);

  $effect(() => {
    draft = maxVisitsStore.value;
  });

  function commit() {
    // 입력을 지워서 빈 문자열이 되면 Svelte가 draft를 NaN으로 바인딩함 - 그 경우
    // 그냥 기존 값으로 되돌린다.
    if (Number.isNaN(draft)) {
      draft = maxVisitsStore.value;
      return;
    }
    maxVisitsStore.set(draft);
  }
</script>

<FormField label={t("settings.maxVisits")} layout="row">
  <NumberInput min={1} step={1} bind:value={draft} onchange={commit} style="width: 96px" />
</FormField>
