mod media_library;
mod media_server;
mod mpv;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            media_library::scan_media_root,
            media_server::get_media_url,
            mpv::mpv_play
        ])
        .run(tauri::generate_context!())
        .expect("failed to run open-course-player");
}
