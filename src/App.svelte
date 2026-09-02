<script lang="ts">
  // 최상위 화면 전환: 별도 라우터 없이 상태값으로 Game/Settings 화면 토글
  // (화면 수가 적어 svelte-spa-router 등은 도입하지 않음)
  import GameScreen from "./lib/screens/GameScreen.svelte";
  import SettingsScreen from "./lib/screens/SettingsScreen.svelte";
  import KeyboardShortcuts from "./lib/components/common/KeyboardShortcuts.svelte";

  let activeScreen: "game" | "settings" = $state("game");

  // Android의 하드웨어/제스처 "뒤로가기"는 WebView 히스토리의 back으로 매핑된다
  // (history entry가 없으면 곧장 앱이 종료/최소화됨) - 그래서 설정 화면을 열 때
  // history entry를 하나 쌓아두고, 뒤로가기 버튼이 발생시키는 popstate에서
  // 메인 화면으로 돌아가도록 한다. X 버튼으로 닫을 때도 history.back()을 호출해
  // 두 경로(하드웨어 뒤로가기 vs 닫기 버튼)에서 히스토리 상태가 항상 일치하게 함.
  function openSettings() {
    activeScreen = "settings";
    history.pushState({ screen: "settings" }, "");
  }

  function closeSettings() {
    history.back();
  }
</script>

<svelte:window onpopstate={() => (activeScreen = "game")} />

<KeyboardShortcuts {activeScreen} onOpenSettings={openSettings} onCloseSettings={closeSettings} />

{#if activeScreen === "game"}
  <GameScreen onOpenSettings={openSettings} />
{:else}
  <SettingsScreen onClose={closeSettings} />
{/if}
