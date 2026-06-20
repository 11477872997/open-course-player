import type { MediaKind, PlayerEngine } from "../types/media";

const mpegTsExtensions = new Set([".ts", ".m2ts", ".mts"]);
const hlsExtensions = new Set([".m3u8"]);
const webVideoExtensions = new Set([".mp4", ".m4v", ".webm", ".ogv", ".sz"]);
const webAudioExtensions = new Set([".mp3", ".wav", ".ogg", ".flac", ".m4a", ".aac", ".opus", ".wma"]);
const mpvExtensions = new Set([".mkv", ".avi", ".flv", ".mov", ".wmv", ".rmvb", ".vob", ".3gp", ".mpeg", ".mpg"]);
const documentExtensions = new Set([".pdf", ".docx", ".doc", ".xlsx", ".xls", ".pptx", ".ppt"]);
const subtitleExtensions = new Set([".srt", ".ass", ".vtt"]);

export function getExtension(fileName: string) {
  const index = fileName.lastIndexOf(".");
  return index >= 0 ? fileName.slice(index).toLowerCase() : "";
}

export function classifyMedia(fileName: string): {
  kind: MediaKind;
  playable: boolean;
  engine: PlayerEngine | "unsupported";
} {
  const ext = getExtension(fileName);

  if (mpegTsExtensions.has(ext)) {
    return { kind: "video", playable: true, engine: "mpegts" };
  }

  if (hlsExtensions.has(ext)) {
    return { kind: "video", playable: true, engine: "easy-player" };
  }

  if (webVideoExtensions.has(ext)) {
    return { kind: "video", playable: true, engine: "web-video" };
  }

  if (webAudioExtensions.has(ext)) {
    return { kind: "audio", playable: true, engine: "web-video" };
  }

  if (mpvExtensions.has(ext)) {
    return { kind: "video", playable: true, engine: "mpv" };
  }

  if (documentExtensions.has(ext)) {
    return { kind: "document", playable: true, engine: "document" };
  }

  if (subtitleExtensions.has(ext)) {
    return { kind: "subtitle", playable: false, engine: "unsupported" };
  }

  return { kind: "unknown", playable: false, engine: "unsupported" };
}

export function describeEngine(engine: PlayerEngine | "unsupported") {
  switch (engine) {
    case "mpegts":
      return "MPEG-TS";
    case "hls":
      return "HLS";
    case "easy-player":
      return "EasyPlayer.js";
    case "mpv":
      return "mpv";
    case "web-video":
      return "\u5185\u7f6e";
    case "document":
      return "\u6587\u6863\u9884\u89c8";
    default:
      return "\u4e0d\u652f\u6301";
  }
}
