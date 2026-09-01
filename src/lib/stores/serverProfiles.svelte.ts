// 등록된 서버 프로필 목록 및 활성 프로필 store
// 실제 저장/로드는 Rust 백엔드(sandbox storage)에 위임, 여기서는 캐시만 보관

import { invoke } from "@tauri-apps/api/core";
import type { NewServerProfile, ServerProfile } from "../types/serverProfile";

function createServerProfilesStore() {
  let profiles = $state<ServerProfile[]>([]);
  let activeProfileId = $state<string | null>(null);

  return {
    get profiles() {
      return profiles;
    },
    get activeProfileId() {
      return activeProfileId;
    },
    async refresh() {
      profiles = await invoke<ServerProfile[]>("list_profiles");
    },
    // id 없이 주면 신규 생성, id를 주면 기존 프로필을 덮어씀
    async save(input: NewServerProfile) {
      const saved = await invoke<ServerProfile>("save_profile", { input });
      const idx = profiles.findIndex((p) => p.id === saved.id);
      if (idx >= 0) {
        profiles[idx] = saved;
      } else {
        profiles.push(saved);
      }
      return saved;
    },
    async remove(id: string) {
      await invoke("delete_profile", { id });
      profiles = profiles.filter((p) => p.id !== id);
      if (activeProfileId === id) {
        activeProfileId = null;
      }
    },
    async setActive(id: string) {
      await invoke("switch_profile", { profileId: id });
      activeProfileId = id;
    },
  };
}

export const serverProfilesStore = createServerProfilesStore();
