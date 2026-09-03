<script lang="ts">
  // 공용 label+입력 패턴 - KomiForm/InstantMoveForm(label 좌 + 입력 우, layout="row")과
  // ServerProfileForm(label 위 + 입력 아래, layout="column")이 각자 갖고 있던 두
  // 레이아웃을 하나로 합침. label을 input의 부모로 그대로 감싸므로(원래
  // ServerProfileForm 방식) for/id를 따로 맞출 필요 없이 두 레이아웃 모두에서
  // 암묵적 label-input 연결이 그대로 성립한다.
  import type { Snippet } from "svelte";

  let {
    label,
    layout = "column",
    hint,
    children,
  }: {
    label: string;
    layout?: "row" | "column";
    hint?: string;
    children: Snippet;
  } = $props();
</script>

<label class="ui-form-field {layout}">
  <span class="label-text">{label}</span>
  {@render children()}
  {#if hint}
    <span class="hint">{hint}</span>
  {/if}
</label>

<style>
  .ui-form-field {
    display: flex;
    gap: var(--space-1);
    font-size: 0.85rem;
  }

  .ui-form-field.column {
    flex-direction: column;
  }

  .ui-form-field.row {
    flex-direction: row;
    align-items: center;
    gap: var(--space-4);
  }

  .ui-form-field.row .label-text {
    flex: 1 1 auto;
  }

  .hint {
    font-size: 0.75rem;
    opacity: 0.7;
  }
</style>
