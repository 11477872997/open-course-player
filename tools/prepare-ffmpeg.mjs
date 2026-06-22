import { copyFileSync, existsSync, mkdirSync, statSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);
const ffmpegPath = resolveFfmpegPath();
const targetName = process.platform === "win32" ? "ffmpeg.exe" : "ffmpeg";
const target = join(root, "src-tauri", "binaries", targetName);

if (!ffmpegPath || !existsSync(ffmpegPath)) {
  throw new Error("没有可用 FFmpeg 二进制，请重新安装依赖或手动安装 FFmpeg");
}

if (!statSync(ffmpegPath).isFile()) {
  throw new Error(`FFmpeg 路径不是文件：${ffmpegPath}`);
}

mkdirSync(dirname(target), { recursive: true });
copyFileSync(ffmpegPath, target);
console.log(`FFmpeg 已准备到 ${target}`);

function resolveFfmpegPath() {
  const candidates = [];

  try {
    candidates.push(require("ffmpeg-static"));
  } catch {
    // Handled by the final error.
  }

  try {
    candidates.push(require("@ffmpeg-installer/ffmpeg").path);
  } catch {
    // Handled by the final error.
  }

  return candidates.find((path) => path && existsSync(path));
}
