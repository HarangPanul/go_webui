// vitest/config가 vite의 defineConfig에 `test` 필드 타입을 얹어 재수출한 것 -
// 이걸 써야 아래 test: {...}가 타입 에러 없이 인식됨.
import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Tauri 규약: 고정 포트 사용, HMR은 모바일 실기기 접속을 고려해 host 노출
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: "0.0.0.0",
  },
  // vitest 설정. jsdom은 DOM(ResizeObserver/keydown 등)에 의존하는 테스트용 -
  // boardGrid.ts처럼 순수 함수만 테스트할 때는 안 쓰이지만 미리 켜둠(rovingFocus.ts
  // 등 DOM 의존 테스트를 나중에 추가할 때 다시 안 건드리도록).
  test: {
    environment: "jsdom",
  },
});
