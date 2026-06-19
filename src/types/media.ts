export type PlayerEngine = "web-video" | "mpegts" | "hls" | "easy-player" | "mpv";

export type MediaKind = "video" | "audio" | "subtitle" | "folder" | "unknown";

export interface MediaTreeNode {
  id: string;
  name: string;
  path: string;
  kind: MediaKind;
  playable: boolean;
  engine: PlayerEngine | "unsupported";
  children?: MediaTreeNode[];
}

export interface SelectedMedia {
  id: string;
  name: string;
  path: string;
  kind: MediaKind;
  engine: PlayerEngine | "unsupported";
  duration?: number | null;
  size?: number | null;
  mime?: string | null;
}

export interface MediaLibraryRoot {
  id: string;
  name: string;
  path: string;
  nodes: MediaTreeNode[];
  totalFiles: number;
  playableFiles: number;
}
