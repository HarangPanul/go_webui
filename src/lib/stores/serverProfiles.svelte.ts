// 등록된 서버 프로필 목록 및 활성 프로필 store
// 실제 저장/로드는 Rust 백엔드(sandbox storage)에 위임, 여기서는 캐시만 보관

import { invoke } from "@tauri-apps/api/core";
import type { NewServerProfile, ServerProfile } from "../types/serverProfile";

function createServerProfilesStore() {
  let profiles = $state<ServerProfile[]>([]);
  let activeProfileId = $state<string | null>(null);
  // 수정 버튼으로 선택된, 폼에 채워 넣을 프로필 (key 원문은 없음 - 아래
  // ServerProfileForm 참고)
  let editingProfile = $state<ServerProfile | null>(null);

  async function refresh() {
    profiles = await invoke<ServerProfile[]>("list_profiles");
  }

  // Settings 화면(ServerProfileList)을 한 번도 연 적이 없어도 "엔진 연결" 키보드
  // 단축키(KeyboardShortcuts.svelte)가 등록된 프로필 목록을 바로 쓸 수 있도록
  // store 생성 시점에 한 번 미리 불러온다. ServerProfileList도 마운트 시 별도로
  // refresh()를 부르지만 멱등이라 문제 없음.
  refresh();

  return {
    get profiles() {
      return profiles;
    },
    get activeProfileId() {
      return activeProfileId;
    },
    get editingProfile() {
      return editingProfile;
    },
    refresh,
    // id 없이 주면 신규 생성, id를 주면 기존 프로필을 덮어씀
    async save(input: NewServerProfile) {
      const saved = await invoke<ServerProfile>("save_profile", { input });
      const idx = profiles.findIndex((p) => p.id === saved.id);
      if (idx >= 0) {
        profiles[idx] = saved;
      } else {
        profiles.push(saved);
      }
      if (editingProfile?.id === saved.id) {
        editingProfile = null;
      }
      return saved;
    },
    async remove(id: string) {
      await invoke("delete_profile", { id });
      profiles = profiles.filter((p) => p.id !== id);
      if (activeProfileId === id) {
        activeProfileId = null;
      }
      if (editingProfile?.id === id) {
        editingProfile = null;
      }
    },
    async setActive(id: string) {
      await invoke("switch_profile", { profileId: id });
      activeProfileId = id;
    },
    startEdit(profile: ServerProfile) {
      editingProfile = profile;
    },
    cancelEdit() {
      editingProfile = null;
    },
  };
}

export const serverProfilesStore = createServerProfilesStore();
