import mpegts from "mpegts.js";

let player: mpegts.Player | null = null;

export function isMpegTsSupported() {
  return mpegts.isSupported();
}

export function playMpegTs(video: HTMLVideoElement, url: string) {
  destroyMpegTs();

  if (!mpegts.isSupported()) {
    throw new Error("当前环境不支持 MPEG-TS 内置播放");
  }

  player = mpegts.createPlayer(
    {
      type: "mpegts",
      isLive: false,
      url
    },
    {
      seekType: "range",
      rangeLoadZeroStart: true,
      accurateSeek: true
    }
  );

  player.attachMediaElement(video);
  player.load();
  player.play();
}

export function destroyMpegTs() {
  if (!player) return;
  player.destroy();
  player = null;
}
