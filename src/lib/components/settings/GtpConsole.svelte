<script lang="ts">
  // 임의 GTP 명령을 보내고 응답을 확인하는 콘솔. kata-analyze 스트리밍 이벤트도
  // 함께 로그로 찍어서 실전 데이터로 파이프라인이 살아있는지 눈으로 바로 확인할
  // 수 있게 함.
  //
  // 여러 프로필이 동시에 연결될 수 있으므로 어느 세션에 보낼지 드롭다운으로 고른다 -
  // 연결된 프로필이 하나뿐이면 그게 자동으로 선택되어 기존과 동일하게 동작한다.
  //
  // 실제 invoke()/이벤트 구독은 전부 gtpStore(stores/gtp.svelte.ts)가 담당 - 이
  // 컴포넌트는 그 결과(log)를 보여주고 입력을 store로 전달하기만 하는 순수 프레젠테이션.
  import { t } from "../../i18n";
  import { connectionStore } from "../../stores/connection.svelte";
  import { serverProfilesStore } from "../../stores/serverProfiles.svelte";
  import { gtpStore } from "../../stores/gtp.svelte";
  import Panel from "../ui/Panel.svelte";
  import TextInput from "../ui/TextInput.svelte";
  import Select from "../ui/Select.svelte";
  import Button from "../ui/Button.svelte";

  let command = $state("");
  let sending = $state(false);
  let selectedProfileId = $state<string | null>(null);

  const connectedProfiles = $derived(
    serverProfilesStore.profiles.filter((p) =>
      connectionStore.connectedProfileIds.includes(p.id),
    ),
  );

  // 선택해둔 프로필이 연결 해제되는 등으로 후보에서 사라지면 첫 번째 연결된 프로필로
  // 자동 대체 - 연결된 프로필이 하나뿐인 흔한 경우엔 사용자가 손댈 일 없이 항상
  // 그게 선택되어 있음.
  $effect(() => {
    if (
      selectedProfileId === null ||
      !connectedProfiles.some((p) => p.id === selectedProfileId)
    ) {
      selectedProfileId = connectedProfiles[0]?.id ?? null;
    }
  });

  async function send() {
    const cmd = command.trim();
    if (!cmd || !selectedProfileId) return;
    sending = true;
    try {
      await gtpStore.send(selectedProfileId, cmd);
    } finally {
      sending = false;
      command = "";
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();
      send();
    }
  }

  let disabled = $derived(!selectedProfileId || sending);
</script>

<div class="gtp-console">
  <Panel class="log">
    {#each gtpStore.log as entry, i (i)}
      <p class="log-entry {entry.kind}">{entry.text}</p>
    {/each}
  </Panel>
  {#if connectedProfiles.length > 1}
    <Select
      value={selectedProfileId ?? ""}
      onchange={(e) => (selectedProfileId = (e.currentTarget as HTMLSelectElement).value)}
    >
      {#each connectedProfiles as profile (profile.id)}
        <option value={profile.id}>{profile.name || profile.host}</option>
      {/each}
    </Select>
  {/if}
  <div class="input-row">
    <TextInput
      bind:value={command}
      onkeydown={handleKeydown}
      disabled={disabled}
      placeholder="name"
    />
    <Button onclick={send} disabled={disabled || !command.trim()}>
      {t("settings.send")}
    </Button>
  </div>
</div>

<style>
  .gtp-console {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  /* 테두리/둥근 모서리/패딩은 ui/Panel.svelte가 담당 - 여기서는 로그 특유의
     스크롤/모노스페이스만 지정 */
  :global(.log) {
    max-height: 200px;
    overflow-y: auto;
    font-family: monospace;
    font-size: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .log-entry {
    margin: 0;
    white-space: pre-wrap;
  }

  .log-entry.sent::before {
    content: "> ";
    opacity: 0.6;
  }

  .log-entry.received {
    opacity: 0.85;
  }

  .log-entry.analysis {
    color: var(--color-primary);
  }

  .log-entry.error {
    color: var(--color-danger);
  }

  .input-row {
    display: flex;
    gap: var(--space-3);
  }

  .input-row :global(.ui-input) {
    flex: 1;
    font-family: monospace;
  }
</style>
