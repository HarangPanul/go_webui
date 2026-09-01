<script lang="ts">
  // 서버 프로필 등록/전환 + SSH Key 입력 화면
  import ServerProfileList from "../components/settings/ServerProfileList.svelte";
  import ServerProfileForm from "../components/settings/ServerProfileForm.svelte";
  import KeybindingsForm from "../components/settings/KeybindingsForm.svelte";

  let { onClose }: { onClose: () => void } = $props();
</script>

<section>
  <ServerProfileList />
  <ServerProfileForm />
  <KeybindingsForm />
  <button class="close-button" onclick={onClose}>Close</button>
</section>

<style>
  .close-button {
    padding: 14px 24px;
    font-size: 1.1rem;
    border: 1px solid #444;
    border-radius: 8px;
    background: transparent;
    color: inherit;
  }
</style>
