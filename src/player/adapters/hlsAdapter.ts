import Hls from "hls.js";

let hls: Hls | null = null;

export function playHls(video: HTMLVideoElement, url: string) {
  destroyHls();

  if (Hls.isSupported()) {
    hls = new Hls();
    hls.loadSource(url);
    hls.attachMedia(video);
    return;
  }

  video.src = url;
}

export function destroyHls() {
  if (!hls) return;
  hls.destroy();
  hls = null;
}
