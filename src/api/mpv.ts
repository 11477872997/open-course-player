import { invoke } from "@tauri-apps/api/core";

export async function playWithMpv(path: string) {
  return invoke<void>("mpv_play", { path });
}
