<script lang="ts">
  // "확인 없이 바로 착수" 옵션 체크박스: 켜면 BoardCanvas가 Confirm Move 버튼/키를
  // 기다리지 않고 마우스는 누르는 순간, 터치스크린은 손을 떼는 순간 바로 착수한다
  // (instantMove.svelte.ts / BoardCanvas.svelte 참고).
  import { t } from "../../i18n";
  import { instantMoveStore } from "../../stores/instantMove.svelte";
  import FormField from "../ui/FormField.svelte";
  import Checkbox from "../ui/Checkbox.svelte";
</script>

<FormField label={t("settings.instantMove")} layout="row">
  <Checkbox
    checked={instantMoveStore.enabled}
    onchange={(evt) => instantMoveStore.set(evt.currentTarget.checked)}
  />
</FormField>
