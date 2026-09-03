mod commands;
mod error;
mod game;
mod gtp;
mod models;
mod services;
mod ssh;
mod state;

use state::AppState;

/// 21개 커맨드 + (커맨드 시그니처에 나타나지 않는) 이벤트 payload 타입들을 모아
/// TypeScript 바인딩(src/lib/generated/bindings.ts)을 만드는 tauri-specta 설정.
/// run()과 디버그 빌드 전용 export 양쪽에서 써야 해서(둘 다 &self만 빌림, Builder가
/// Clone이라 소유권 문제도 없음) 별도 함수로 뺐다.
///
/// error_handling은 기본값(Result - 모든 커맨드가 `{status:"ok"|"error", ...}`
/// 객체를 resolve하는 방식)이 아니라 Throw를 명시적으로 골랐다: 기존 프런트 전체가
/// invoke()가 reject하면 catch(e)로 잡는 방식으로 짜여 있어서(try/catch 수십 곳),
/// Throw여야 그 계약을 그대로 유지한 채 성공 값 타입만 얻을 수 있다.
fn specta_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new()
        .commands(tauri_specta::collect_commands![
            commands::ssh::connect_ssh,
            commands::ssh::disconnect_ssh,
            commands::ssh::list_local_ssh_keys,
            commands::ssh::load_local_ssh_key,
            commands::gtp::send_gtp_command,
            commands::gtp::start_kata_analyze,
            commands::gtp::set_engine_color,
            commands::gtp::get_engine_colors,
            commands::gtp::get_komi,
            commands::gtp::set_komi,
            commands::profile::list_profiles,
            commands::profile::save_profile,
            commands::profile::delete_profile,
            commands::profile::switch_profile,
            commands::game::get_board_state,
            commands::game::confirm_move,
            commands::game::pass_move,
            commands::game::go_back,
            commands::game::go_forward,
            commands::game::remove_last_move,
            commands::game::toggle_turn,
        ])
        .error_handling(tauri_specta::ErrorHandlingMode::Throw)
        // BoardSnapshot.size/nodeId/ancestorChain, MoveInfo.x/y 등이 Rust에서는
        // usize다 - specta는 usize/u64를 기본적으로 BigInt 취급해 export를 막는데,
        // 여기서는 전부 보드 좌표/노드 인덱스라 실질적으로 JS의 안전한 정수 범위를
        // 벗어날 일이 없으므로(19x19 보드, 아무리 긴 대국이라도 수천~수만 수 단위)
        // number로 캐스팅해도 안전하다.
        .dangerously_cast_bigints_to_number()
        // "connection-status"/"kata-analyze" 이벤트 payload - 커맨드 인자/반환값
        // 어디에도 나타나지 않아 커맨드 등록만으로는 자동으로 안 잡히므로 직접 등록.
        // 이벤트는 (frontend listen()/backend app.emit() 그대로 유지, Event derive
        // 기반 typed-event API는 아직 안 씀) 타입만 여기서 내보낸다.
        .typ::<gtp::process::StatusPayload>()
        .typ::<gtp::process::KataAnalyzeEvent>()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let specta_builder = specta_builder();

    #[cfg(debug_assertions)]
    specta_builder
        .export(
            specta_typescript::Typescript::default(),
            "../src/lib/generated/bindings.ts",
        )
        .expect("failed to export typescript bindings");

    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(specta_builder.invoke_handler())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::specta_builder;

    // run()의 export 호출은 실제 GUI 앱을 띄우는 경로 안에 있어 헤드리스로 확인하기
    // 어려우므로, 여기서 직접 같은 export를 호출해 (1) bindings.ts를 실제로 갱신하고
    // (2) 타입 그래프가 항상 문제없이 export되는지 회귀 테스트로 검증한다.
    #[test]
    fn typescript_bindings_export_succeeds() {
        specta_builder()
            .export(
                specta_typescript::Typescript::default(),
                "../src/lib/generated/bindings.ts",
            )
            .expect("TypeScript 바인딩 export 실패");
    }
}
