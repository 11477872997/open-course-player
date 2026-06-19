import { invoke } from "@tauri-apps/api/core";

export interface PlaybackHistory {
  roots: string[];
  activeRootPath?: string | null;
  lastMediaPath?: string | null;
  updatedAt?: number | null;
}

export function loadPlaybackHistory() {
  return invoke<PlaybackHistory>("load_playback_history");
}

export function savePlaybackHistory(history: PlaybackHistory) {
  return invoke<void>("save_playback_history", { history });
}
