// 데스크탑 로컬 SSH key 감지 store - SshKeyInput.svelte가 직접 하던
// invoke("list_local_ssh_keys"/"load_local_ssh_key", ...) 호출을 감싼다. 모바일
// 등 미지원 플랫폼에서는 list_local_ssh_keys 자체가 에러를 반환하므로 keys는 빈
// 배열로 남고(SshKeyInput이 그 경우 감지 UI 자체를 숨김), load()의 실패는 호출자가
// 처리한다(SshKeyInput의 loadError).
import { invoke } from "@tauri-apps/api/core";
import type { LocalSshKeyInfo } from "../generated/bindings";

function createSshKeysStore() {
  let keys = $state<LocalSshKeyInfo[]>([]);

  invoke<LocalSshKeyInfo[]>("list_local_ssh_keys")
    .then((k) => (keys = k))
    .catch(() => (keys = []));

  return {
    get keys() {
      return keys;
    },
    load(path: string): Promise<string> {
      return invoke<string>("load_local_ssh_key", { path });
    },
  };
}

export const sshKeysStore = createSshKeysStore();
