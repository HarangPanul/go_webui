mod commands;
mod game;
mod gtp;
mod models;
mod ssh;
mod state;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::ssh::connect_ssh,
            commands::ssh::disconnect_ssh,
            commands::gtp::send_gtp_command,
            commands::profile::list_profiles,
            commands::profile::save_profile,
            commands::profile::switch_profile,
            commands::game::get_board_state,
            commands::game::confirm_move,
            commands::game::go_back,
            commands::game::remove_last_move,
            commands::game::toggle_turn,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
