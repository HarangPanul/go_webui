<script lang="ts">
  // "확인 없이 바로 착수" 옵션 체크박스: 켜면 BoardCanvas가 Confirm Move 버튼/키를
  // 기다리지 않고 마우스는 누르는 순간, 터치스크린은 손을 떼는 순간 바로 착수한다
  // (instantMove.svelte.ts / BoardCanvas.svelte 참고).
  import { t } from "../../i18n";
  import { instantMoveStore } from "../../stores/instantMove.svelte";
</script>

<div class="instant-move-form">
  <label for="instant-move-checkbox">{t("settings.instantMove")}</label>
  <input
    id="instant-move-checkbox"
    type="checkbox"
    checked={instantMoveStore.enabled}
    onchange={(evt) => instantMoveStore.set(evt.currentTarget.checked)}
  />
</div>

<style>
  .instant-move-form {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  label {
    flex: 1 1 auto;
  }

  input[type="checkbox"] {
    width: 20px;
    height: 20px;
  }
</style>
