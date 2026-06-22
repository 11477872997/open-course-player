import { invoke } from "@tauri-apps/api/core";

export interface MediaSourceInfo {
  url: string;
  mime: string;
  size: number;
  duration?: number | null;
}

export interface SzMediaDiagnostic {
  container: string;
  videoCodec?: string | null;
  audioCodec?: string | null;
  videoSampleCount: number;
  audioSampleCount: number;
  duration?: number | null;
  firstVideoSampleSize?: number | null;
  firstVideoSampleHead?: string | null;
  firstAudioSampleSize?: number | null;
  firstAudioSampleHead?: string | null;
  hasStandardProtectionBoxes: boolean;
  looksLikeStandardH264: boolean;
  looksLikeStandardAac: boolean;
  mdatEntropy?: number | null;
  conclusion: string;
}

export function createMediaSource(path: string) {
  return invoke<MediaSourceInfo>("create_media_source", { path });
}

export function transcodeMediaToCompatibleMp4(path: string) {
  return invoke<MediaSourceInfo>("transcode_media_to_compatible_mp4", { path });
}

export function diagnoseSzMedia(path: string) {
  return invoke<SzMediaDiagnostic>("diagnose_sz_media", { path });
}
