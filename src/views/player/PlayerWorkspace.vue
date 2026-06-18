<script setup lang="ts">
import { computed, ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ElMessage } from "element-plus";
import { Close, Minus, Refresh, Search, SwitchButton } from "@element-plus/icons-vue";
import LibraryList from "./components/LibraryList.vue";
import FileTree from "./components/FileTree.vue";
import PlaybackSurface from "./components/PlaybackSurface.vue";
import PlayerControls from "./components/PlayerControls.vue";
import { scanMediaRoot } from "../../api/mediaLibrary";
import { describeEngine } from "../../player/mediaTypes";
import { choosePlayback } from "../../player/playbackRouter";
import { useLibraryStore } from "../../store/modules/library";
import type { MediaLibraryRoot, MediaTreeNode, SelectedMedia } from "../../types/media";

const library = useLibraryStore();
const appWindow = isTauri() ? getCurrentWindow() : null;
const loadingRootId = ref("");
const searchKeyword = ref("");
const playbackMessage = ref("添加本地文件夹后开始播放");

const activeRoot = computed(() => library.activeRoot);
const selectedEngineName = computed(() =>
  library.selectedMedia ? describeEngine(library.selectedMedia.engine) : "未选择"
);
const currentNodes = computed(() => library.activeNodes);
const loading = computed(() => Boolean(loadingRootId.value));

const filteredNodes = computed(() => {
  const keyword = searchKeyword.value.trim().toLowerCase();
  if (!keyword) return currentNodes.value;
  return filterTree(currentNodes.value, keyword);
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

async function loadFolder(rootPath: string) {
  if (!rootPath) return;

  loadingRootId.value = rootPath;
  try {
    const nodes = await scanMediaRoot(rootPath);
    library.upsertRoot(rootPath, nodes);
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
}

function removeRoot(root: MediaLibraryRoot) {
  library.removeRoot(root.id);
  if (library.selectedMedia?.path.startsWith(root.path)) {
    library.clearSelectedMedia();
  }
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
    engine: node.engine
  };

  library.selectMedia(media);
  const decision = choosePlayback(media);
  playbackMessage.value = decision.reason;
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

function startDrag() {
  void appWindow?.startDragging();
}

function minimizeWindow() {
  void appWindow?.minimize();
}

function toggleMaximizeWindow() {
  void appWindow?.toggleMaximize();
}

function closeWindow() {
  void appWindow?.close();
}
</script>

<template>
  <main class="player-shell">
    <section class="player-window">
      <header class="window-toolbar" @dblclick="toggleMaximizeWindow">
        <div class="drag-region" @mousedown.left="startDrag" />

        <div class="brand-mini">
          <img src="../../assets/brand/cd-player.png" alt="" />
          <strong>Open Course Player</strong>
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
          <el-button :icon="SwitchButton" @click="toggleMaximizeWindow" />
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
        />

        <section class="player-column">
          <PlaybackSurface
            :media="library.selectedMedia"
            :message="playbackMessage"
            :engine-name="selectedEngineName"
            @add-folder="chooseFolder"
          />
          <PlayerControls :disabled="!library.selectedMedia" />
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
          />
        </aside>
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
  position: relative;
  display: grid;
  grid-template-columns: 180px minmax(220px, 1fr) auto;
  align-items: center;
  gap: 16px;
  padding: 0 12px;
  border-bottom: 1px solid var(--ocp-dark-border);
  background: rgba(12, 18, 27, 0.86);
}

.drag-region {
  position: absolute;
  inset: 0;
  z-index: 0;
}

.brand-mini,
.global-search,
.window-actions {
  position: relative;
  z-index: 1;
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
  .player-shell {
    padding: 0;
  }

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
