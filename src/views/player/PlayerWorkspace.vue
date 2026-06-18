<script setup lang="ts">
import { computed, ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { ElMessage } from "element-plus";
import { Refresh, Search, VideoPlay } from "@element-plus/icons-vue";
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
const loadingRootId = ref("");
const searchKeyword = ref("");
const playbackMessage = ref("添加课程文件夹后开始浏览");

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
    title: "添加课程文件夹"
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
    playbackMessage.value = nodes.length ? "选择课程文件开始播放" : "该目录没有发现可播放文件";
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
    library.selectedMedia = null;
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
  return nodes
    .map((node) => {
      const children = node.children ? filterTree(node.children, keyword) : [];
      const matched = node.name.toLowerCase().includes(keyword);
      if (!matched && !children.length) return null;
      return {
        ...node,
        children: children.length ? children : node.children && matched ? node.children : undefined
      };
    })
    .filter((node): node is MediaTreeNode => Boolean(node));
}
</script>

<template>
  <main class="player-workspace">
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

    <section class="browser-pane">
      <header class="browser-header">
        <div class="header-title">
          <strong>{{ activeRoot?.name || "未选择资料库" }}</strong>
          <span>{{ activeRoot?.path || "添加一个或多个文件夹后开始浏览课程" }}</span>
        </div>
        <el-tooltip content="刷新当前资料库" placement="bottom">
          <el-button
            :icon="Refresh"
            :disabled="!activeRoot || loading"
            @click="refreshRoot()"
          />
        </el-tooltip>
      </header>

      <div class="search-row">
        <el-input
          v-model="searchKeyword"
          :prefix-icon="Search"
          clearable
          placeholder="搜索当前资料库"
        />
      </div>

      <FileTree
        :nodes="filteredNodes"
        :loading="loading"
        :active-media-id="library.selectedMedia?.id || ''"
        @select="handleSelect"
      />
    </section>

    <section class="playback-pane">
      <header class="playback-header">
        <div class="media-title">
          <strong>{{ library.selectedMedia?.name || "未选择文件" }}</strong>
          <span>{{ library.selectedMedia?.path || "从左侧文件树选择一个课程视频" }}</span>
        </div>
        <el-button :icon="VideoPlay" type="primary" :disabled="!library.selectedMedia">
          播放
        </el-button>
      </header>

      <PlaybackSurface
        :media="library.selectedMedia"
        :message="playbackMessage"
        :engine-name="selectedEngineName"
        @add-folder="chooseFolder"
      />
      <PlayerControls :disabled="!library.selectedMedia" />
    </section>
  </main>
</template>

<style scoped lang="scss">
.player-workspace {
  display: grid;
  width: 100vw;
  height: 100vh;
  grid-template-columns: 220px minmax(300px, 360px) minmax(0, 1fr);
  background: var(--ocp-bg);
  color: var(--ocp-text);
}

.browser-pane,
.playback-pane {
  min-width: 0;
  min-height: 0;
}

.browser-pane {
  display: grid;
  grid-template-rows: auto auto minmax(0, 1fr);
  border-right: 1px solid var(--ocp-border);
  background: var(--ocp-surface);
}

.browser-header,
.playback-header {
  display: flex;
  min-height: 56px;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 10px 14px;
  border-bottom: 1px solid var(--ocp-border);
}

.header-title,
.media-title {
  min-width: 0;
}

.header-title strong,
.media-title strong {
  overflow: hidden;
  display: block;
  color: var(--ocp-text);
  font-size: 14px;
  font-weight: 700;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.header-title span,
.media-title span {
  overflow: hidden;
  display: block;
  margin-top: 4px;
  color: var(--ocp-text-muted);
  font-size: 12px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.search-row {
  padding: 10px 12px;
  border-bottom: 1px solid #edf1f5;
}

.playback-pane {
  display: grid;
  grid-template-rows: auto minmax(0, 1fr) auto;
  background: #eef2f7;
}

@media (max-width: 980px) {
  .player-workspace {
    grid-template-columns: 220px minmax(0, 1fr);
    grid-template-rows: minmax(0, 46vh) minmax(0, 54vh);
  }

  .playback-pane {
    grid-column: 1 / -1;
  }
}
</style>
