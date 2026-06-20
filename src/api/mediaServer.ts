import { invoke } from "@tauri-apps/api/core";

export interface MediaSourceInfo {
  url: string;
  mime: string;
  size: number;
  duration?: number | null;
}

export function createMediaSource(path: string) {
  return invoke<MediaSourceInfo>("create_media_source", { path });
}

export function transcodeMediaToCompatibleMp4(path: string) {
  return invoke<MediaSourceInfo>("transcode_media_to_compatible_mp4", { path });
}
