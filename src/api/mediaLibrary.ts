import { invoke } from "@tauri-apps/api/core";
import type { MediaTreeNode } from "../types/media";

export async function scanMediaRoot(rootPath: string) {
  return invoke<MediaTreeNode[]>("scan_media_root", { rootPath });
}

export async function getMediaUrl(path: string) {
  return invoke<string>("get_media_url", { path });
}
