<script lang="ts">
  // 공용 버튼. variant="primary"는 확정/저장처럼 꽉 찬 강조 버튼, variant="danger"는
  // 되돌릴 수 없는 파괴적 동작(마지막 수 제거, 프로필 삭제 등)에 쓴다.
  //
  // active는 별도의 ToggleButton 없이 "지금 강조돼야 하는 상태"를 표시하는 prop -
  // 엔진 자동 착수/Analysis/Ownership 켬, 키보드 단축키 재배정 대기 중 등 종류가
  // 달라도 전부 같은 파란 강조(--color-accent)로 표시한다.
  //
  // shape="circle"은 GameControls의 다음 착수 색 전환 원형 아이콘 버튼 전용.
  import type { Snippet } from "svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";

  type Props = HTMLButtonAttributes & {
    variant?: "default" | "primary" | "danger";
    active?: boolean;
    shape?: "rect" | "circle";
    children: Snippet;
  };

  let {
    variant = "default",
    active = false,
    shape = "rect",
    type = "button",
    class: className = "",
    children,
    ...rest
  }: Props = $props();
</script>

<button
  {type}
  class="ui-button {variant} {className}"
  class:active
  class:circle={shape === "circle"}
  {...rest}
>
  {@render children()}
</button>

<style>
  .ui-button {
    padding: var(--space-4) var(--space-6);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: transparent;
    color: inherit;
    font: inherit;
  }

  .ui-button:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .ui-button.primary {
    border-color: var(--color-primary);
    background: var(--color-primary);
    color: #fff;
  }

  .ui-button.danger {
    border-color: var(--color-danger);
    color: var(--color-danger);
  }

  .ui-button.active {
    border-color: var(--color-accent);
    color: var(--color-accent);
  }

  .ui-button.circle {
    flex: 0 0 auto;
    min-width: 0;
    width: 44px;
    height: 44px;
    padding: var(--space-2);
    border-radius: var(--radius-full);
    display: flex;
    align-items: center;
    justify-content: center;
  }
</style>
