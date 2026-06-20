mod file_location;
mod media_library;
mod media_server;
mod mpv;
mod playback_history;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            file_location::reveal_path,
            media_library::scan_media_root,
            media_server::create_media_source,
            media_server::create_subtitle_source,
            media_server::find_subtitle_tracks,
            media_server::transcode_media_to_compatible_mp4,
            mpv::mpv_play,
            playback_history::load_playback_history,
            playback_history::save_playback_history
        ])
        .run(tauri::generate_context!())
        .expect("failed to run open-course-player");
}
