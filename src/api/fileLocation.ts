import { invoke } from "@tauri-apps/api/core";

export function revealPathInFileManager(path: string) {
  return invoke<void>("reveal_path", { path });
}
