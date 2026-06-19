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
