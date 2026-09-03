<script lang="ts">
  // 공용 label+입력 패턴. layout="row"(label 좌 + 입력 우)와 layout="column"
  // (label 위 + 입력 아래) 두 배치를 지원한다. label이 input을 그대로 감싸므로
  // for/id를 따로 맞추지 않아도 두 레이아웃 모두에서 암묵적 label-input 연결이
  // 성립한다.
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
