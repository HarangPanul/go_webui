// 등록된 서버 프로필 목록 및 활성 프로필 store
// 실제 저장/로드는 Rust 백엔드(sandbox storage)에 위임, 여기서는 캐시만 보관

import type { ServerProfile } from "../types/serverProfile";

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
    // TODO: invoke("list_profiles"/"save_profile"/"switch_profile")
  };
}

export const serverProfilesStore = createServerProfilesStore();
