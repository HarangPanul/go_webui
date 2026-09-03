<script lang="ts">
  // 공용 테두리+둥근 모서리 박스 - GtpConsole의 로그 창, ServerProfileList의 프로필
  // 항목/연결 상태 줄이 각자 갖고 있던 테두리 스타일을 하나로 합침. class는 그대로
  // 전달해 각 호출부가 자기만의 추가 레이아웃(overflow, flex 방향 등)을 덧붙일 수
  // 있게 함.
  import type { Snippet } from "svelte";
  import type { HTMLAttributes } from "svelte/elements";

  type Props = HTMLAttributes<HTMLDivElement> & {
    children: Snippet;
  };

  let { class: className = "", children, ...rest }: Props = $props();
</script>

<div class="ui-panel {className}" {...rest}>
  {@render children()}
</div>

<style>
  .ui-panel {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: var(--space-4);
  }
</style>
