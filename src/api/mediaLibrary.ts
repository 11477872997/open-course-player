import { invoke } from "@tauri-apps/api/core";
import type { MediaTreeNode } from "../types/media";

export async function scanMediaRoot(rootPath: string) {
  return invoke<MediaTreeNode[]>("scan_media_root", { rootPath });
}
