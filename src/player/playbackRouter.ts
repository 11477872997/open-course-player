import type { PlayerEngine, SelectedMedia } from "../types/media";

export interface PlaybackDecision {
  engine: PlayerEngine | "unsupported";
  reason: string;
}

export function choosePlayback(media: SelectedMedia): PlaybackDecision {
  if (media.engine === "unsupported") {
    return {
      engine: "unsupported",
      reason: "当前文件格式暂未接入播放器"
    };
  }

  return {
    engine: media.engine,
    reason: "根据扩展名选择优先播放器"
  };
}
