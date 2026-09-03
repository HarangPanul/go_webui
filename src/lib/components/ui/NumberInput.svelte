<script lang="ts">
  // 공용 숫자 입력. TextInput과 같은 톤(테두리/둥근 모서리/투명 배경).
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
