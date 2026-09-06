<script lang="ts">
  // "임시 선택 위치 십자선 표시" 옵션 체크박스: 끄면 BoardCanvas가 착수 예정 위치에
  // 그리던 빨간 십자선을 더 이상 그리지 않는다(pendingCrosshair.svelte.ts /
  // BoardCanvas.svelte 참고). 미리보기 돌 자체는 이 설정과 무관하게 항상 표시됨.
  import { t } from "../../i18n";
  import { pendingCrosshairStore } from "../../stores/pendingCrosshair.svelte";
  import FormField from "../ui/FormField.svelte";
  import Checkbox from "../ui/Checkbox.svelte";
</script>

<FormField label={t("settings.pendingCrosshair")} layout="row">
  <Checkbox
    checked={pendingCrosshairStore.enabled}
    onchange={(evt) => pendingCrosshairStore.set(evt.currentTarget.checked)}
  />
</FormField>
