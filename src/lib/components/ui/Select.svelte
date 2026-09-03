<script lang="ts">
  // 공용 드롭다운 - SshKeyInput의 감지된 SSH key 목록 select 스타일을 그대로 옮김.
  // option들은 호출부가 children으로 직접 렌더링(값 목록의 형태가 컴포넌트마다
  // 다르므로 이 컴포넌트는 틀만 제공).
  import type { Snippet } from "svelte";
  import type { HTMLSelectAttributes } from "svelte/elements";

  type Props = Omit<HTMLSelectAttributes, "value"> & {
    value?: string;
    children: Snippet;
  };

  let { value = $bindable(""), children, ...rest }: Props = $props();
</script>

<select class="ui-input" bind:value {...rest}>
  {@render children()}
</select>

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
