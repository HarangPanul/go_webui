<script lang="ts">
  // 공용 숫자 입력 - KomiForm(step=0.5)/ServerProfileForm(포트, min/max)이 각자
  // 갖고 있던 입력창 스타일을 TextInput과 동일한 톤으로 통일.
  import type { HTMLInputAttributes } from "svelte/elements";

  type Props = Omit<HTMLInputAttributes, "type" | "value"> & {
    value?: number;
  };

  let { value = $bindable(0), ...rest }: Props = $props();
</script>

<input type="number" class="ui-input" bind:value {...rest} />

<style>
  .ui-input {
    padding: var(--space-3) var(--space-4);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: transparent;
    color: inherit;
    font: inherit;
  }
</style>
