<script lang="ts">
  // 공용 버튼: 지금까지 GameControls/ServerProfileForm/ServerProfileList/
  // KeybindingsForm/CollapsibleSection/TopBar 등에 각자 복사돼 있던 버튼 스타일을
  // 하나로 합침. variant="primary"는 확정/저장처럼 꽉 찬 강조 버튼(원래
  // button.primary), variant="danger"는 되돌릴 수 없는 파괴적 동작(원래
  // button.danger, ServerProfileList의 삭제 버튼 - 두 곳에서 서로 다른 빨강
  // #a33/#d66과 #c0392b로 갈라져 있던 걸 --color-danger 하나로 통일).
  //
  // active는 별도의 ToggleButton을 만드는 대신 기존 button.toggle.active(엔진
  // 자동 착수/Analysis/Ownership 켬)와 KeybindingsForm의 button.key.listening(키
  // 재배정 대기 중)을 같은 "지금 강조돼야 하는 상태" 패턴으로 접어 넣은 것 -
  // 시각적으로 파란 강조(--color-accent)로 통일된다(listening은 원래 초록이었지만
  // 같은 의미의 상태이므로 하나로 합침).
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
