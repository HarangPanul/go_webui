<script lang="ts">
  // 공용 한 줄 텍스트 입력 - ServerProfileForm/GtpConsole 등에 복사돼 있던 입력창
  // 스타일(테두리/둥근 모서리/투명 배경)을 하나로 합침.
  import type { HTMLInputAttributes } from "svelte/elements";

  type Props = Omit<HTMLInputAttributes, "type" | "value"> & {
    value?: string;
  };

  let { value = $bindable(""), ...rest }: Props = $props();
</script>

<input type="text" class="ui-input" bind:value {...rest} />

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
