<script lang="ts">
  // 덤(komi) 입력: 값이 바뀌면(엔터/포커스 아웃) komiStore.set()이 저장과 동시에
  // 연결된 엔진에도 바로 반영함(komi.svelte.ts 참고). type="number" 입력에
  // bind:value를 쓰면 Svelte가 자동으로 숫자로 변환해주므로 draft도 number로 둔다.
  // oninput이 아니라 onchange(포커스 아웃/엔터 시점)로만 커밋해서 타이핑 중간값마다
  // 매번 백엔드에 보내지 않는다.
  import { t } from "../../i18n";
  import { komiStore } from "../../stores/komi.svelte";

  // 입력 중인 값은 별도 로컬 상태로 두고, komiStore.value가 바뀔 때(예: 앱 시작
  // 시 백엔드에서 값을 가져온 직후) 그 값으로 다시 채워 넣는다.
  let draft = $state(komiStore.value);

  $effect(() => {
    draft = komiStore.value;
  });

  function commit() {
    // 입력을 지워서 빈 문자열이 되면 Svelte가 draft를 NaN으로 바인딩함 - 그 경우
    // 그냥 기존 값으로 되돌린다.
    if (Number.isNaN(draft)) {
      draft = komiStore.value;
      return;
    }
    komiStore.set(draft);
  }
</script>

<div class="komi-form">
  <label for="komi-input">{t("settings.komi")}</label>
  <input id="komi-input" type="number" step="0.5" bind:value={draft} onchange={commit} />
</div>

<style>
  .komi-form {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  label {
    flex: 1 1 auto;
  }

  input {
    width: 96px;
    padding: 6px 10px;
    border: 1px solid #444;
    border-radius: 6px;
    background: transparent;
    color: inherit;
  }
</style>
