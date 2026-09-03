<script lang="ts">
  // 공용 여러 줄 텍스트 입력 - SshKeyInput의 key 붙여넣기 textarea 스타일을 그대로
  // 옮김(monospace + resize: vertical).
  import type { HTMLTextareaAttributes } from "svelte/elements";

  type Props = Omit<HTMLTextareaAttributes, "value"> & {
    value?: string;
  };

  let { value = $bindable(""), ...rest }: Props = $props();
</script>

<textarea class="ui-textarea" bind:value {...rest}></textarea>

<style>
  .ui-textarea {
    width: 100%;
    padding: var(--space-3) var(--space-4);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: transparent;
    color: inherit;
    font-family: monospace;
    font-size: 0.75rem;
    resize: vertical;
  }
</style>
