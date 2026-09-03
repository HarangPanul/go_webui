// 캔버스 backing store를 부모 요소의 CSS 크기 x devicePixelRatio로 맞추고,
// ResizeObserver로 부모 크기가 바뀔 때마다 다시 맞춘 뒤 onResize를 호출하는 Svelte
// action. BoardCanvas.svelte/AnalysisOverlay.svelte/OwnershipOverlay.svelte 3곳에
// 거의 그대로 복사돼 있던 DPR 대응 리사이즈 + ResizeObserver 준비/해제 로직을 하나로
// 모았다. 각 컴포넌트는 이 action을 <canvas>에 붙이고 onResize에서 자기 draw()를
// 부르기만 하면 됨 - "크기 맞추기"만 이 action의 책임이고, 그 외 무엇이 바뀔 때 다시
// 그릴지(보드 상태, 분석 결과 등)는 각 컴포넌트가 자기 $effect로 계속 따로 챙긴다.
export function canvasLayer(node: HTMLCanvasElement, onResize: () => void) {
  const parent = node.parentElement;
  if (!parent) return;

  const resize = () => {
    const dpr = window.devicePixelRatio || 1;
    const cssSize = parent.clientWidth;
    node.width = Math.round(cssSize * dpr);
    node.height = Math.round(cssSize * dpr);
    onResize();
  };

  resize();
  const observer = new ResizeObserver(resize);
  observer.observe(parent);

  return {
    destroy() {
      observer.disconnect();
    },
  };
}
