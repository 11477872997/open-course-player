import { invoke } from "@tauri-apps/api/core";

export interface SubtitleTrackInfo {
  label: string;
  path: string;
  format: string;
}

export interface SubtitleSourceInfo {
  url: string;
  mime: string;
  format: string;
}

export function findSubtitleTracks(path: string) {
  return invoke<SubtitleTrackInfo[]>("find_subtitle_tracks", { path });
}

export function createSubtitleSource(path: string) {
  return invoke<SubtitleSourceInfo>("create_subtitle_source", { path });
}
