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
import PlaybackSurface from "./components/PlaybackSurface.vue";
import PlayerControls from "./components/PlayerControls.vue";
import { revealPathInFileManager } from "../../api/fileLocation";
import { scanMediaRoot } from "../../api/mediaLibrary";
import { createMediaSource } from "../../api/mediaServer";
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

const activeRoot = computed(() => library.activeRoot);
const selectedEngineName = computed(() =>
  library.selectedMedia ? describeEngine(library.selectedMedia.engine) : "未选择"
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
    void loadSelectedMedia(media);
  }
);

onBeforeUnmount(() => {
  document.removeEventListener("fullscreenchange", syncFullscreenState);
  document.removeEventListener("click", closeContextMenu);
  stopPlaybackSync();
  stopHistorySaveTimer();
  destroyPlaybackAdapters();
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

  const media: SelectedMedia = {
    id: node.id,
    name: node.name,
    path: node.path,
    kind: node.kind,
    engine: node.engine
  };

  library.selectMedia(media);
  scheduleHistorySave();
  const decision = choosePlayback(media);
  playbackMessage.value = decision.reason;
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

async function loadSelectedMedia(media: SelectedMedia | null) {
  destroyPlaybackAdapters();
  stopPlaybackSync();
  mediaSourceUrl.value = "";
  subtitleUrl.value = "";
  subtitleLabel.value = "";
  subtitleTracks.value = [];
  playing.value = false;
  currentTime.value = 0;
  duration.value = 0;

  if (!media) {
    playbackMessage.value = "添加本地文件夹后开始播放";
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

  const sourcePath = normalizeLocalPath(media.path);
  const source = isTauri()
    ? await createMediaSource(sourcePath)
    : { url: sourcePath, duration: null, size: null, mime: null };
  const sourceUrl = source.url;
  mediaSourceUrl.value = sourceUrl;
  const measuredDuration = normalizeDuration(source.duration ?? undefined, 0);
  if (measuredDuration > 0) duration.value = measuredDuration;
  const playableMedia: SelectedMedia = {
    ...media,
    duration: measuredDuration || media.duration,
    size: source.size ?? media.size,
    mime: source.mime ?? media.mime
  };
  playbackMessage.value = "媒体已加载";
  await loadAutoSubtitle(playableMedia);
  await nextTick();
  attachPlaybackAdapter(playableMedia, sourceUrl);
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

function attachPlaybackAdapter(media: SelectedMedia, sourceUrl: string) {
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
      const seconds = normalizeDuration(mediaInfo.duration, video.duration);
      if (seconds > 0) duration.value = seconds;
    });
    mpegtsPlayer.on(mpegts.Events.ERROR, (_type: string, detail: string) => {
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
  if (seconds > 0) duration.value = seconds;
}

function syncTime() {
  const video = mediaElement.value;
  if (!video) return;
  currentTime.value = video.currentTime;
  const seconds = normalizeDuration(video.duration, duration.value);
  if (seconds > 0) duration.value = seconds;
  playing.value = !video.paused && !video.ended;
}

function setPlayingState(value: boolean) {
  playing.value = value;
}

async function handleMediaError() {
  const media = library.selectedMedia;
  const element = mediaElement.value;
  if (!media) return;

  const reason = mediaErrorReason(element?.error?.code);
  playbackMessage.value = `内置播放失败：${reason}`;
  ElMessage.warning(playbackMessage.value);

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
          <PlaybackSurface
            :media="library.selectedMedia"
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
            @media-error="handleMediaError"
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
