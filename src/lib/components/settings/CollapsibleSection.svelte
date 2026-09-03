<script lang="ts">
  // Settings 화면의 각 영역(대국 설정/키보드 단축키/GTP 콘솔 등)을 제목 버튼 뒤로
  // 접어 넣는 공용 컨테이너. ServerProfileForm이 "엔진 추가" 버튼 뒤에 숨어있는
  // 것과 같은 패턴을 여러 섹션에 재사용하기 위해 분리함 - 기본은 접힌 상태.
  import type { Snippet } from "svelte";
  import Button from "../ui/Button.svelte";

  let {
    title,
    defaultExpanded = false,
    children,
  }: {
    title: string;
    defaultExpanded?: boolean;
    children: Snippet;
  } = $props();

  let expanded = $state(defaultExpanded);
</script>

<div class="collapsible-section">
  <Button
    class="section-header"
    aria-expanded={expanded}
    onclick={() => (expanded = !expanded)}
  >
    <span class="arrow" class:open={expanded}>▶</span>
    <span class="title">{title}</span>
  </Button>
  {#if expanded}
    <div class="section-body">
      {@render children()}
    </div>
  {/if}
</div>

<style>
  .collapsible-section {
    display: flex;
    flex-direction: column;
    width: 100%;
  }

  /* 테두리/색은 ui/Button.svelte가 담당 - 여기서는 이 헤더 특유의 배치만 지정 */
  :global(.section-header) {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    width: 100%;
    font-size: 0.95rem;
    text-align: left;
  }

  .arrow {
    display: inline-block;
    font-size: 0.7em;
    transition: transform 0.15s ease;
  }

  .arrow.open {
    transform: rotate(90deg);
  }

  .title {
    flex: 1 1 auto;
    font-weight: 600;
  }

  .section-body {
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
    padding: var(--space-6) var(--space-2) var(--space-2);
  }
</style>
