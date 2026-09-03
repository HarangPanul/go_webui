<script lang="ts">
  // 공용 체크박스 (20x20).
  import type { HTMLInputAttributes } from "svelte/elements";

  type Props = Omit<HTMLInputAttributes, "type" | "checked"> & {
    checked?: boolean;
  };

  let { checked = $bindable(false), ...rest }: Props = $props();
</script>

<input type="checkbox" class="ui-checkbox" bind:checked {...rest} />

<style>
  .ui-checkbox {
    width: 20px;
    height: 20px;
  }
</style>
