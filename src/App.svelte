<script lang="ts">
  // 최상위 화면 전환: 별도 라우터 없이 상태값으로 Game/Settings 화면 토글
  // (화면 수가 적어 svelte-spa-router 등은 도입하지 않음)
  import GameScreen from "./lib/screens/GameScreen.svelte";
  import SettingsScreen from "./lib/screens/SettingsScreen.svelte";

  let activeScreen: "game" | "settings" = $state("game");
</script>

{#if activeScreen === "game"}
  <GameScreen onOpenSettings={() => (activeScreen = "settings")} />
{:else}
  <SettingsScreen onClose={() => (activeScreen = "game")} />
{/if}
