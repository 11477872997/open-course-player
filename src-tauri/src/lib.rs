mod file_location;
mod media_library;
mod mpv;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            file_location::reveal_path,
            media_library::scan_media_root,
            mpv::mpv_play
        ])
        .run(tauri::generate_context!())
        .expect("failed to run open-course-player");
}
