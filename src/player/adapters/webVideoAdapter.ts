export interface WebVideoAdapterOptions {
  mediaElement: HTMLVideoElement;
  url: string;
}

export function playWebVideo({ mediaElement, url }: WebVideoAdapterOptions) {
  mediaElement.src = url;
  return mediaElement.play();
}
