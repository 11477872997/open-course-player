<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ElMessage } from "element-plus";
import { Close, FullScreen, Minus, Refresh, Search } from "@element-plus/icons-vue";
import Hls from "hls.js";
import mpegts from "mpegts.js";
import LibraryList from "./components/LibraryList.vue";
import FileTree from "./components/FileTree.vue";
import DocumentPreview from "./components/DocumentPreview.vue";
import PlaybackSurface from "./components/PlaybackSurface.vue";
import PlayerControls from "./components/PlayerControls.vue";
import { revealPathInFileManager } from "../../api/fileLocation";
import { scanMediaRoot } from "../../api/mediaLibrary";
import { createMediaSource, transcodeMediaToCompatibleMp4 } from "../../api/mediaServer";
import { playWithMpv } from "../../api/mpv";
import { loadPlaybackHistory, savePlaybackHistory } from "../../api/playbackHistory";
import { createSubtitleSource, findSubtitleTracks } from "../../api/subtitles";
import { describeEngine } from "../../player/mediaTypes";
import { choosePlayback } from "../../player/playbackRouter";
import { useLibraryStore } from "../../store/modules/library";
import type { MediaLibraryRoot, MediaTreeNode, SelectedMedia } from "../../types/media";
import type { SubtitleTrackInfo } from "../../api/subtitles";

const library = useLibraryStore();
const appWindow = isTauri() ? getCurrentWindow() : null;
const loadingRootId = ref("");
const searchKeyword = ref("");
const playbackMessage = ref("添加本地文件夹后开始播放");
const mediaElement = ref<HTMLMediaElement | null>(null);
const mediaSourceUrl = ref("");
const playbackMedia = ref<SelectedMedia | null>(null);
const subtitleUrl = ref("");
const subtitleLabel = ref("");
const subtitleTracks = ref<SubtitleTrackInfo[]>([]);
const playing = ref(false);
const currentTime = ref(0);
const duration = ref(0);
const volume = ref(0.8);
const playbackRate = ref(1);
const playerFullscreen = ref(false);
const historyReady = ref(false);
const contextMenu = ref<{
  visible: boolean;
  x: number;
  y: number;
  target: MediaTreeNode | MediaLibraryRoot | null;
  type: "file" | "library";
}>({
  visible: false,
  x: 0,
  y: 0,
  target: null,
  type: "file"
});

let hlsPlayer: Hls | null = null;
let mpegtsPlayer: mpegts.Player | null = null;
let playbackSyncTimer: number | null = null;
let historySaveTimer: number | null = null;
let mediaLoadTimer: number | null = null;
let mediaLoadToken = 0;
let mediaLoadInFlight = false;
let queuedMediaLoad: SelectedMedia | null | undefined;
const transcodedFallbackPaths = new Set<string>();

const activeRoot = computed(() => library.activeRoot);
const activePlaybackMedia = computed(() => playbackMedia.value || library.selectedMedia);
const selectedEngineName = computed(() =>
  activePlaybackMedia.value ? describeEngine(activePlaybackMedia.value.engine) : "未选择"
);
const currentNodes = computed(() => library.activeNodes);
const loading = computed(() => Boolean(loadingRootId.value));
const playableNodes = computed(() => flattenPlayableNodes(currentNodes.value));

const filteredNodes = computed(() => {
  const keyword = searchKeyword.value.trim().toLowerCase();
  if (!keyword) return currentNodes.value;
  return filterTree(currentNodes.value, keyword);
});

watch(
  () => library.selectedMedia,
  (media) => {
    scheduleSelectedMediaLoad(media ? normalizeSelectedMedia(media) : null);
  }
);

onBeforeUnmount(() => {
  document.removeEventListener("fullscreenchange", syncFullscreenState);
  document.removeEventListener("click", closeContextMenu);
  stopPlaybackSync();
  stopHistorySaveTimer();
  stopMediaLoadTimer();
  destroyPlaybackAdapters();
  resetMediaElement();
});

onMounted(() => {
  document.addEventListener("fullscreenchange", syncFullscreenState);
  document.addEventListener("click", closeContextMenu);
  void restorePlaybackHistory();
});

async function chooseFolder() {
  const selected = await open({
    directory: true,
    multiple: true,
    title: "添加本地文件夹"
  });

  if (!selected) return;
  const paths = Array.isArray(selected) ? selected : [selected];
  for (const path of paths) {
    await loadFolder(path);
  }
}

async function loadFolder(rootPath: string, options: { persist?: boolean } = {}) {
  if (!rootPath) return;

  loadingRootId.value = rootPath;
  try {
    const nodes = await scanMediaRoot(rootPath);
    library.upsertRoot(rootPath, nodes);
    if (options.persist !== false) scheduleHistorySave();
    playbackMessage.value = nodes.length ? "选择文件开始播放" : "该目录没有发现可播放文件";
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    ElMessage.error(message);
  } finally {
    loadingRootId.value = "";
  }
}

function refreshRoot(root = activeRoot.value) {
  if (!root) return;
  void loadFolder(root.path);
}

function selectRoot(root: MediaLibraryRoot) {
  library.setActiveRoot(root.id);
  scheduleHistorySave();
}

function removeRoot(root: MediaLibraryRoot) {
  library.removeRoot(root.id);
  if (library.selectedMedia?.path.startsWith(root.path)) {
    library.clearSelectedMedia();
  }
  scheduleHistorySave();
}

function handleSelect(node: MediaTreeNode) {
  if (node.kind === "folder") return;

  if (!node.playable) {
    playbackMessage.value = "该文件暂未接入播放器";
    return;
  }

  const media: SelectedMedia = normalizeSelectedMedia({
    id: node.id,
    name: node.name,
    path: node.path,
    kind: node.kind,
    engine: node.engine
  });

  library.selectMedia(media);
  scheduleHistorySave();
  const decision = choosePlayback(media);
  playbackMessage.value = decision.reason;
}

function normalizeSelectedMedia(media: SelectedMedia): SelectedMedia {
  if (isDocumentFile(media.name || media.path)) {
    return {
      ...media,
      kind: "document",
      engine: "document"
    };
  }

  return media;
}

function isDocumentFile(name: string) {
  return /\.(pdf|docx?|xlsx?|pptx?)$/i.test(name);
}

function isSzCourseVideo(name: string) {
  return /\.sz$/i.test(name);
}

async function openMediaLocation(target?: { path: string } | null) {
  const path = target?.path || library.selectedMedia?.path;
  if (!path) {
    ElMessage.info("请先选择一个文件");
    return;
  }

  try {
    await revealPathInFileManager(normalizeLocalPath(path));
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    ElMessage.warning(message);
  }
}

function scheduleSelectedMediaLoad(media: SelectedMedia | null) {
  queuedMediaLoad = media;
  const token = ++mediaLoadToken;

  stopMediaLoadTimer();
  resetPlaybackState();
  playbackMessage.value = media ? "正在加载媒体..." : "添加本地文件夹后开始播放";

  mediaLoadTimer = window.setTimeout(() => {
    mediaLoadTimer = null;
    void flushSelectedMediaLoad(token);
  }, 120);
}

async function flushSelectedMediaLoad(token: number) {
  if (mediaLoadInFlight) return;
  const media = queuedMediaLoad ?? null;
  queuedMediaLoad = undefined;
  mediaLoadInFlight = true;

  try {
    await loadSelectedMedia(media, token);
  } finally {
    mediaLoadInFlight = false;
    if (queuedMediaLoad !== undefined) {
      const nextToken = mediaLoadToken;
      void flushSelectedMediaLoad(nextToken);
    }
  }
}

function stopMediaLoadTimer() {
  if (mediaLoadTimer === null) return;
  window.clearTimeout(mediaLoadTimer);
  mediaLoadTimer = null;
}

function isCurrentLoad(token: number) {
  return token === mediaLoadToken;
}

function resetPlaybackState() {
  destroyPlaybackAdapters();
  stopPlaybackSync();
  resetMediaElement();
  mediaSourceUrl.value = "";
  playbackMedia.value = null;
  subtitleUrl.value = "";
  subtitleLabel.value = "";
  subtitleTracks.value = [];
  playing.value = false;
  currentTime.value = 0;
  duration.value = 0;
}

async function loadSelectedMedia(media: SelectedMedia | null, token = ++mediaLoadToken) {
  resetPlaybackState();

  if (!media) {
    playbackMessage.value = "添加本地文件夹后开始播放";
    return;
  }

  if (isSzCourseVideo(media.name || media.path)) {
    await loadSzAsCompatibleMp4(media, token);
    return;
  }

  if (media.engine === "mpv") {
    try {
      await playWithMpv(normalizeLocalPath(media.path));
      playbackMessage.value = "已调用 mpv 兜底播放";
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      playbackMessage.value = message;
      ElMessage.warning(message);
    }
    return;
  }

  if (media.engine === "unsupported") {
    playbackMessage.value = "当前文件格式暂未接入播放器";
    return;
  }

  if (media.engine === "document") {
    const sourcePath = normalizeLocalPath(media.path);
    const source = isTauri()
      ? await createMediaSource(sourcePath)
      : { url: sourcePath, duration: null, size: null, mime: null };
    if (!isCurrentLoad(token)) return;

    mediaSourceUrl.value = source.url;
    playbackMedia.value = {
      ...media,
      duration: null,
      size: source.size ?? media.size,
      mime: source.mime ?? media.mime
    };
    playbackMessage.value = "文档已加载";
    return;
  }

  const sourcePath = normalizeLocalPath(media.path);
  const source = isTauri()
    ? await createMediaSource(sourcePath)
    : { url: sourcePath, duration: null, size: null, mime: null };
  if (!isCurrentLoad(token)) return;

  const sourceUrl = source.url;
  mediaSourceUrl.value = sourceUrl;
  const measuredDuration = normalizeDuration(source.duration ?? undefined, 0);
  if (measuredDuration > 0) duration.value = measuredDuration;
  const resolvedPlayback = resolvePlaybackFromMime(source.mime, media);
  const playableMedia: SelectedMedia = {
    ...media,
    kind: resolvedPlayback.kind,
    engine: resolvedPlayback.engine,
    duration: measuredDuration || media.duration,
    size: source.size ?? media.size,
    mime: source.mime ?? media.mime
  };
  playbackMessage.value = "媒体已加载";
  playbackMedia.value = playableMedia;
  await loadAutoSubtitle(playableMedia);
  if (!isCurrentLoad(token)) return;

  await nextTick();
  if (!isCurrentLoad(token)) return;

  attachPlaybackAdapter(playableMedia, sourceUrl, token);
}

async function loadSzAsCompatibleMp4(media: SelectedMedia, token: number) {
  const sourcePath = normalizeLocalPath(media.path);
  playbackMessage.value = "正在将 .sz 转为兼容 MP4...";
  ElMessage.info(playbackMessage.value);

  try {
    const source = isTauri()
      ? await transcodeMediaToCompatibleMp4(sourcePath)
      : { url: sourcePath, duration: media.duration ?? null, size: media.size ?? null, mime: "video/mp4" };
    if (!isCurrentLoad(token)) return;

    transcodedFallbackPaths.add(sourcePath);
    mediaSourceUrl.value = source.url;
    const measuredDuration = normalizeDuration(source.duration ?? undefined, 0);
    if (measuredDuration > 0) duration.value = measuredDuration;

    const playableMedia: SelectedMedia = {
      ...media,
      kind: "video",
      engine: "web-video",
      duration: measuredDuration || media.duration,
      size: source.size ?? media.size,
      mime: source.mime ?? "video/mp4"
    };
    playbackMedia.value = playableMedia;
    playbackMessage.value = "已转为兼容 MP4";

    await loadAutoSubtitle(playableMedia);
    if (!isCurrentLoad(token)) return;

    await nextTick();
    if (!isCurrentLoad(token)) return;

    attachPlaybackAdapter(playableMedia, source.url, token);
    return true;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    playbackMessage.value = `.sz 转码失败：${message}`;
    ElMessage.warning(playbackMessage.value);
    return false;
  }
}

function resolvePlaybackFromMime(mime: string | null | undefined, media: SelectedMedia) {
  const normalized = (mime || "").toLowerCase();

  if (normalized.includes("video/mp2t")) {
    return { kind: "video" as const, engine: "mpegts" as const };
  }

  if (
    normalized.includes("application/vnd.apple.mpegurl") ||
    normalized.includes("mpegurl")
  ) {
    return { kind: "video" as const, engine: "easy-player" as const };
  }

  if (normalized.startsWith("video/")) {
    return { kind: "video" as const, engine: "web-video" as const };
  }

  if (normalized.startsWith("audio/")) {
    return { kind: "audio" as const, engine: "web-video" as const };
  }

  if (
    normalized.includes("pdf") ||
    normalized.includes("wordprocessingml") ||
    normalized.includes("spreadsheetml") ||
    normalized.includes("presentationml") ||
    normalized.includes("msword") ||
    normalized.includes("ms-excel") ||
    normalized.includes("ms-powerpoint")
  ) {
    return { kind: "document" as const, engine: "document" as const };
  }

  return { kind: media.kind, engine: media.engine };
}

async function loadAutoSubtitle(media: SelectedMedia) {
  if (media.kind !== "video" || !isTauri()) return;

  try {
    const tracks = await findSubtitleTracks(normalizeLocalPath(media.path));
    subtitleTracks.value = tracks;
    const preferred = tracks.find((track) => track.format === "vtt" || track.format === "srt");
    if (!preferred) return;

    const source = await createSubtitleSource(normalizeLocalPath(preferred.path));
    subtitleUrl.value = source.url;
    subtitleLabel.value = preferred.label;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.warn("字幕加载失败", message);
  }
}

function attachPlaybackAdapter(media: SelectedMedia, sourceUrl: string, token = mediaLoadToken) {
  if (!isCurrentLoad(token)) return;

  const video = mediaElement.value;
  if (!video) return;

  video.volume = volume.value;
  video.playbackRate = playbackRate.value;

  if (media.engine === "mpegts") {
    video.removeAttribute("src");
    video.load();

    if (!mpegts.isSupported() || !mpegts.getFeatureList().msePlayback) {
      playbackMessage.value = "当前环境不支持 MPEG-TS 内置播放";
      return;
    }

    mpegtsPlayer = mpegts.createPlayer({
      type: "mpegts",
      url: sourceUrl,
      isLive: false,
      duration: media.duration ? media.duration * 1000 : undefined,
      filesize: media.size ?? undefined
    }, {
      seekType: "range",
      rangeLoadZeroStart: true,
      accurateSeek: true
    });
    mpegtsPlayer.on(mpegts.Events.MEDIA_INFO, (mediaInfo: mpegts.MSEPlayerMediaInfo) => {
      if (!isCurrentLoad(token)) return;
      const seconds = normalizeDuration(mediaInfo.duration, video.duration);
      if (shouldAcceptElementDuration(seconds)) duration.value = seconds;
    });
    mpegtsPlayer.on(mpegts.Events.ERROR, (_type: string, detail: string) => {
      if (!isCurrentLoad(token)) return;
      playbackMessage.value = `MPEG-TS 播放失败：${detail}`;
      ElMessage.warning(playbackMessage.value);
    });
    mpegtsPlayer.attachMediaElement(video);
    mpegtsPlayer.load();
    playbackMessage.value = "MPEG-TS 已加载";
    startPlaybackSync();
    return;
  }

  if (media.engine === "easy-player" && Hls.isSupported()) {
    video.removeAttribute("src");
    video.load();

    hlsPlayer = new Hls();
    hlsPlayer.loadSource(sourceUrl);
    hlsPlayer.attachMedia(video);
    playbackMessage.value = "HLS 已加载";
    startPlaybackSync();
    return;
  }

  video.src = sourceUrl;
  video.load();
  startPlaybackSync();
}

function destroyPlaybackAdapters() {
  hlsPlayer?.destroy();
  hlsPlayer = null;
  mpegtsPlayer?.destroy();
  mpegtsPlayer = null;
}

function resetMediaElement() {
  const element = mediaElement.value;
  if (!element) return;
  element.pause();
  element.removeAttribute("src");
  while (element.firstChild) {
    element.removeChild(element.firstChild);
  }
  element.load();
}

function setMediaElement(element: HTMLMediaElement | null) {
  mediaElement.value = element;
  syncMetadata();
  syncTime();
}

async function togglePlayPause() {
  const video = mediaElement.value;
  if (!video || !library.selectedMedia) return;

  if (video.paused) {
    try {
      await video.play();
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      ElMessage.warning(`播放失败：${message}`);
    }
  } else {
    video.pause();
  }
}

function seekTo(value: number) {
  const video = mediaElement.value;
  if (!video || !Number.isFinite(value)) return;
  video.currentTime = value;
  currentTime.value = value;
}

function setVolume(value: number) {
  volume.value = Math.min(1, Math.max(0, value));
  if (mediaElement.value) mediaElement.value.volume = volume.value;
}

function setPlaybackRate(value: number) {
  playbackRate.value = value;
  if (mediaElement.value) mediaElement.value.playbackRate = value;
}

function syncMetadata() {
  const video = mediaElement.value;
  if (!video) return;
  const seconds = normalizeDuration(video.duration, duration.value);
  if (shouldAcceptElementDuration(seconds)) duration.value = seconds;
}

function syncTime() {
  const video = mediaElement.value;
  if (!video) return;
  currentTime.value = video.currentTime;
  const seconds = normalizeDuration(video.duration, duration.value);
  if (shouldAcceptElementDuration(seconds)) duration.value = seconds;
  playing.value = !video.paused && !video.ended;
}

function shouldAcceptElementDuration(seconds: number) {
  if (!Number.isFinite(seconds) || seconds <= 0) return false;
  const media = activePlaybackMedia.value;
  if (media?.engine === "mpegts" && duration.value > 0 && seconds < duration.value * 0.8) {
    return false;
  }
  return true;
}

function setPlayingState(value: boolean) {
  playing.value = value;
}

async function handleMediaErrorWithFallback() {
  const media = library.selectedMedia;
  const element = mediaElement.value;
  if (!media) return;

  const reason = mediaErrorReason(element?.error?.code);
  playbackMessage.value = `内置播放失败：${reason}`;
  ElMessage.warning(playbackMessage.value);

  const token = mediaLoadToken;
  if (await tryTranscodedFallback(media, token)) {
    return;
  }

  if (isSzCourseVideo(media.name || media.path)) {
    playbackMessage.value = `.sz 转码后的 MP4 仍无法内置播放：${reason}`;
    ElMessage.warning(playbackMessage.value);
    return;
  }

  if (media.engine !== "mpv") {
    try {
      await playWithMpv(normalizeLocalPath(media.path));
      playbackMessage.value = "已切换到外部播放器兜底";
      ElMessage.success(playbackMessage.value);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      playbackMessage.value = `外部播放器兜底失败：${message}`;
      ElMessage.warning(playbackMessage.value);
    }
  }
}

async function tryTranscodedFallback(media: SelectedMedia, token: number) {
  if (!isTauri() || !isSzCourseVideo(media.name || media.path)) return false;

  const sourcePath = normalizeLocalPath(media.path);
  if (transcodedFallbackPaths.has(sourcePath)) return false;
  transcodedFallbackPaths.add(sourcePath);

  playbackMessage.value = "内置无法解码，正在用 FFmpeg 转为兼容 MP4...";
  ElMessage.info(playbackMessage.value);

  try {
    destroyPlaybackAdapters();
    stopPlaybackSync();
    resetMediaElement();
    const loaded = await loadSzAsCompatibleMp4(media, token);
    if (loaded) {
      ElMessage.success("已转为兼容 MP4，继续内置播放");
      return true;
    }
    return false;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    playbackMessage.value = `FFmpeg 转码失败：${message}`;
    ElMessage.warning(playbackMessage.value);
    return false;
  }
}

async function handleMediaError() {
  const media = library.selectedMedia;
  const element = mediaElement.value;
  if (!media) return;

  const reason = mediaErrorReason(element?.error?.code);
  playbackMessage.value = `内置播放失败：${reason}`;
  ElMessage.warning(playbackMessage.value);

  if (isSzCourseVideo(media.name || media.path)) return;

  if (media.engine !== "mpv") {
    try {
      await playWithMpv(normalizeLocalPath(media.path));
      playbackMessage.value = "已切换到 mpv 兜底播放";
    } catch {
      // mpv 是可选兜底；没有安装时保留内置播放器的错误提示。
    }
  }
}

function startPlaybackSync() {
  stopPlaybackSync();
  playbackSyncTimer = window.setInterval(syncTime, 300);
}

function stopPlaybackSync() {
  if (playbackSyncTimer === null) return;
  window.clearInterval(playbackSyncTimer);
  playbackSyncTimer = null;
}

function playPrevious() {
  playAdjacent(-1);
}

function playNext() {
  playAdjacent(1);
}

function playAdjacent(direction: -1 | 1) {
  const current = library.selectedMedia;
  const items = playableNodes.value;
  if (!current || !items.length) return;

  const index = items.findIndex((item) => item.id === current.id);
  const nextIndex = index < 0 ? 0 : (index + direction + items.length) % items.length;
  handleSelect(items[nextIndex]);
}

async function requestPlayerFullscreen(element: HTMLElement) {
  try {
    await element.requestFullscreen();
    playerFullscreen.value = true;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    ElMessage.warning(`全屏失败：${message}`);
  }
}

async function exitPlayerFullscreen() {
  if (!document.fullscreenElement) return;
  await document.exitFullscreen();
}

function syncFullscreenState() {
  playerFullscreen.value = Boolean(document.fullscreenElement);
}

function normalizeDuration(primary: number | undefined, fallback = 0) {
  const value = Number(primary);
  if (Number.isFinite(value) && value > 0) {
    return value > 24 * 60 * 60 ? value / 1000 : value;
  }

  return Number.isFinite(fallback) && fallback > 0 ? fallback : 0;
}

function normalizeLocalPath(path: string) {
  return path.replace(/^\\\\\?\\/, "");
}

function mediaErrorReason(code?: number) {
  switch (code) {
    case MediaError.MEDIA_ERR_ABORTED:
      return "播放被中断";
    case MediaError.MEDIA_ERR_NETWORK:
      return "媒体读取失败";
    case MediaError.MEDIA_ERR_DECODE:
      return "当前编码无法解码";
    case MediaError.MEDIA_ERR_SRC_NOT_SUPPORTED:
      return "当前格式或路径不被内置播放器支持";
    default:
      return "未知媒体错误";
  }
}

function filterTree(nodes: MediaTreeNode[], keyword: string): MediaTreeNode[] {
  const result: MediaTreeNode[] = [];

  for (const node of nodes) {
    const children = node.children ? filterTree(node.children, keyword) : [];
    const matched = node.name.toLowerCase().includes(keyword);

    if (matched || children.length) {
      result.push({
        ...node,
        children: children.length ? children : node.children && matched ? node.children : undefined
      });
    }
  }

  return result;
}

function flattenPlayableNodes(nodes: MediaTreeNode[]) {
  const result: MediaTreeNode[] = [];
  const walk = (items: MediaTreeNode[]) => {
    for (const item of items) {
      if (item.playable && item.kind !== "folder") result.push(item);
      if (item.children?.length) walk(item.children);
    }
  };
  walk(nodes);
  return result;
}

function startDrag(event: MouseEvent) {
  if ((event.target as HTMLElement).closest("button, input, .el-input, .window-actions")) return;
  if (!appWindow) return;

  appWindow.startDragging().catch((error) => {
    const message = error instanceof Error ? error.message : String(error);
    console.warn("窗口拖动失败", message);
  });
}

function minimizeWindow() {
  if (!appWindow) return;
  appWindow.minimize().catch(showWindowActionError);
}

async function toggleFullscreenWindow() {
  if (!appWindow) return;
  try {
    const fullscreen = await appWindow.isFullscreen();
    await appWindow.setFullscreen(!fullscreen);
  } catch (error) {
    showWindowActionError(error);
  }
}

function closeWindow() {
  if (!appWindow) return;
  appWindow.close().catch(showWindowActionError);
}

function showFileContextMenu(node: MediaTreeNode, event: MouseEvent) {
  contextMenu.value = {
    visible: true,
    x: event.clientX,
    y: event.clientY,
    target: node,
    type: "file"
  };
}

function showLibraryContextMenu(root: MediaLibraryRoot, event: MouseEvent) {
  contextMenu.value = {
    visible: true,
    x: event.clientX,
    y: event.clientY,
    target: root,
    type: "library"
  };
}

function closeContextMenu() {
  contextMenu.value.visible = false;
}

async function openContextMenuLocation() {
  const target = contextMenu.value.target;
  closeContextMenu();
  await openMediaLocation(target);
}

async function restorePlaybackHistory() {
  if (!isTauri()) {
    historyReady.value = true;
    return;
  }

  try {
    const history = await loadPlaybackHistory();
    const roots = history.roots.filter(Boolean);
    if (!roots.length) {
      historyReady.value = true;
      return;
    }

    for (const root of roots) {
      await loadFolder(root, { persist: false });
    }

    if (history.activeRootPath) {
      library.setActiveRoot(history.activeRootPath);
    }

    if (history.lastMediaPath) {
      const restored = findNodeByPath(library.roots.flatMap((root) => root.nodes), history.lastMediaPath);
      if (restored?.playable && restored.kind !== "folder") {
        handleSelect(restored);
      }
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.warn("恢复历史记录失败", message);
  } finally {
    historyReady.value = true;
    scheduleHistorySave();
  }
}

function scheduleHistorySave() {
  if (!isTauri() || !historyReady.value) return;

  stopHistorySaveTimer();
  historySaveTimer = window.setTimeout(() => {
    void persistPlaybackHistory();
  }, 250);
}

function stopHistorySaveTimer() {
  if (historySaveTimer === null) return;
  window.clearTimeout(historySaveTimer);
  historySaveTimer = null;
}

async function persistPlaybackHistory() {
  try {
    await savePlaybackHistory({
      roots: library.roots.map((root) => root.path),
      activeRootPath: library.activeRoot?.path ?? null,
      lastMediaPath: library.selectedMedia?.path ?? null
    });
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.warn("保存历史记录失败", message);
  }
}

function findNodeByPath(nodes: MediaTreeNode[], path: string): MediaTreeNode | null {
  for (const node of nodes) {
    if (node.path === path) return node;
    if (node.children?.length) {
      const matched = findNodeByPath(node.children, path);
      if (matched) return matched;
    }
  }

  return null;
}

function showWindowActionError(error: unknown) {
  const message = error instanceof Error ? error.message : String(error);
  ElMessage.warning(`窗口操作失败：${message}`);
}
</script>

<template>
  <main class="player-shell">
    <section class="player-window">
      <header class="window-toolbar" data-tauri-drag-region @mousedown.left="startDrag">
        <div class="brand-mini" data-tauri-drag-region>
          <img src="../../assets/brand/cd-player.png" alt="" />
          <strong data-tauri-drag-region>Open Course Player</strong>
        </div>

        <el-input
          v-model="searchKeyword"
          class="global-search"
          :prefix-icon="Search"
          clearable
          placeholder="搜索文件..."
        />

        <div class="window-actions">
          <el-button :icon="Minus" @click="minimizeWindow" />
          <el-button :icon="FullScreen" @click="toggleFullscreenWindow" />
          <el-button class="close-button" :icon="Close" @click="closeWindow" />
        </div>
      </header>

      <div class="workspace-grid">
        <LibraryList
          :roots="library.roots"
          :active-root-id="activeRoot?.id || ''"
          :loading="loading"
          :total-playable-files="library.totalPlayableFiles"
          @add="chooseFolder"
          @refresh="refreshRoot"
          @select="selectRoot"
          @remove="removeRoot"
          @context-menu="showLibraryContextMenu"
        />

        <section class="player-column">
          <DocumentPreview
            v-if="activePlaybackMedia?.kind === 'document'"
            :media="activePlaybackMedia"
            :source-url="mediaSourceUrl"
            :message="playbackMessage"
            @add-folder="chooseFolder"
            @open-location="openMediaLocation()"
          />
          <PlaybackSurface
            v-else
            :media="activePlaybackMedia"
            :source-url="mediaSourceUrl"
            :subtitle-url="subtitleUrl"
            :subtitle-label="subtitleLabel"
            :message="playbackMessage"
            :engine-name="selectedEngineName"
            :playing="playing"
            :current-time="currentTime"
            :duration="duration"
            :volume="volume"
            :playback-rate="playbackRate"
            :fullscreen="playerFullscreen"
            @add-folder="chooseFolder"
            @previous="playPrevious"
            @next="playNext"
            @play-pause="togglePlayPause"
            @fullscreen="requestPlayerFullscreen"
            @exit-fullscreen="exitPlayerFullscreen"
            @media-mounted="setMediaElement"
            @loaded-metadata="syncMetadata"
            @duration-change="syncMetadata"
            @media-error="handleMediaErrorWithFallback"
            @time-update="syncTime"
            @seek="seekTo"
            @volume="setVolume"
            @rate="setPlaybackRate"
            @play="setPlayingState(true)"
            @pause="setPlayingState(false)"
            @ended="playNext"
            @open-location="openMediaLocation()"
          />
          <PlayerControls
            v-if="activePlaybackMedia?.kind !== 'document'"
            :disabled="!library.selectedMedia || !mediaSourceUrl"
            :playing="playing"
            :current-time="currentTime"
            :duration="duration"
            :volume="volume"
            :playback-rate="playbackRate"
            @play-pause="togglePlayPause"
            @previous="playPrevious"
            @next="playNext"
            @seek="seekTo"
            @volume="setVolume"
            @rate="setPlaybackRate"
          />
        </section>

        <aside class="chapter-panel">
          <header class="chapter-header">
            <strong>章节列表</strong>
            <el-tooltip content="刷新当前资料库" placement="bottom">
              <el-button
                :icon="Refresh"
                :disabled="!activeRoot || loading"
                @click="refreshRoot()"
              />
            </el-tooltip>
          </header>
          <FileTree
            :nodes="filteredNodes"
            :loading="loading"
            :active-media-id="library.selectedMedia?.id || ''"
            @select="handleSelect"
            @context-menu="showFileContextMenu"
          />
        </aside>
      </div>

      <div
        v-if="contextMenu.visible"
        class="context-menu"
        :style="{ left: `${contextMenu.x}px`, top: `${contextMenu.y}px` }"
        @click.stop
      >
        <button type="button" @click="openContextMenuLocation">
          打开文件所在位置
        </button>
      </div>
    </section>
  </main>
</template>

<style scoped lang="scss">
.player-shell {
  display: grid;
  width: 100vw;
  height: 100vh;
  overflow: hidden;
  padding: 0;
  background:
    radial-gradient(circle at 18% 0%, rgba(59, 130, 246, 0.16), transparent 34%),
    radial-gradient(circle at 82% 18%, rgba(14, 165, 233, 0.08), transparent 32%),
    var(--ocp-bg);
  color: var(--ocp-text-inverse);
}

.player-window {
  display: grid;
  width: 100%;
  height: 100%;
  min-width: 0;
  min-height: 0;
  grid-template-rows: 50px minmax(0, 1fr);
  overflow: hidden;
  border: 0;
  border-radius: 0;
  background: rgba(15, 23, 34, 0.94);
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.04);
}

.window-toolbar {
  display: grid;
  grid-template-columns: 180px minmax(220px, 1fr) auto;
  align-items: center;
  gap: 16px;
  padding: 0 12px;
  border-bottom: 1px solid var(--ocp-dark-border);
  background: rgba(12, 18, 27, 0.86);
}

.brand-mini {
  display: flex;
  min-width: 0;
  align-items: center;
  gap: 8px;
}

.brand-mini img {
  width: 22px;
  height: 22px;
}

.brand-mini strong {
  overflow: hidden;
  color: var(--ocp-text-inverse);
  font-size: 13px;
  font-weight: 700;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.global-search {
  width: min(100%, 440px);
  justify-self: center;
}

.global-search :deep(.el-input__wrapper) {
  min-height: 32px;
  background: rgba(255, 255, 255, 0.045);
  box-shadow: 0 0 0 1px rgba(148, 163, 184, 0.16) inset;
}

.global-search :deep(.el-input__inner) {
  color: var(--ocp-text-inverse);
}

.window-actions {
  display: flex;
  align-items: center;
  gap: 4px;
}

.window-actions :deep(.el-button) {
  width: 28px;
  height: 28px;
  min-height: 28px;
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--ocp-text-inverse-muted);
}

.window-actions :deep(.el-button:hover) {
  background: rgba(255, 255, 255, 0.08);
  color: var(--ocp-text-inverse);
}

.window-actions :deep(.close-button:hover) {
  background: rgba(239, 68, 68, 0.22);
  color: #ffffff;
}

.workspace-grid {
  display: grid;
  min-width: 0;
  min-height: 0;
  grid-template-columns: clamp(180px, 14vw, 280px) minmax(0, 1fr) clamp(250px, 22vw, 420px);
}

.player-column {
  display: grid;
  min-width: 0;
  min-height: 0;
  grid-template-rows: minmax(0, 1fr) auto;
  padding: 12px;
  border-right: 1px solid var(--ocp-dark-border);
}

.chapter-panel {
  display: grid;
  min-width: 0;
  min-height: 0;
  grid-template-rows: 46px minmax(0, 1fr);
  background: rgba(12, 18, 27, 0.42);
}

.chapter-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 12px;
  border-bottom: 1px solid rgba(148, 163, 184, 0.14);
}

.chapter-header strong {
  color: var(--ocp-text-inverse);
  font-size: 13px;
}

.chapter-header :deep(.el-button) {
  width: 28px;
  height: 28px;
  min-height: 28px;
  padding: 0;
  border-color: rgba(148, 163, 184, 0.16);
  background: rgba(255, 255, 255, 0.04);
  color: var(--ocp-text-inverse-muted);
}

.context-menu {
  position: fixed;
  z-index: 40;
  min-width: 154px;
  padding: 6px;
  border: 1px solid rgba(148, 163, 184, 0.22);
  border-radius: 7px;
  background: rgba(15, 23, 34, 0.96);
  box-shadow: 0 18px 48px rgba(0, 0, 0, 0.32);
}

.context-menu button {
  display: flex;
  width: 100%;
  align-items: center;
  min-height: 30px;
  padding: 0 10px;
  border: 0;
  border-radius: 5px;
  background: transparent;
  color: #d8e5f7;
  cursor: pointer;
  font-size: 12px;
  text-align: left;
}

.context-menu button:hover {
  background: rgba(59, 130, 246, 0.16);
  color: #ffffff;
}

@media (min-width: 1500px) {
  .window-toolbar {
    grid-template-columns: clamp(180px, 14vw, 280px) minmax(260px, 1fr) auto;
  }
}

@media (max-width: 1180px) {
  .workspace-grid {
    grid-template-columns: 180px minmax(0, 1fr) 250px;
  }

  .window-toolbar {
    grid-template-columns: 180px minmax(220px, 1fr) auto;
  }
}

@media (max-width: 980px) {
  .player-window {
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    border-radius: 0;
  }

  .workspace-grid {
    grid-template-columns: 170px minmax(0, 1fr);
  }

  .chapter-panel {
    display: none;
  }
}
</style>
