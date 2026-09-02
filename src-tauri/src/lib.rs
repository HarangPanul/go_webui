mod commands;
mod error;
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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
