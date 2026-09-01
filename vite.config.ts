import { defineConfig } from "vite";
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
});
