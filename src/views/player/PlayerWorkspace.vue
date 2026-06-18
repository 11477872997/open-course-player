<script setup lang="ts">
import { computed, ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { ElMessage } from "element-plus";
import { FolderOpened, Monitor, Refresh, VideoPlay } from "@element-plus/icons-vue";
import FileTree from "./components/FileTree.vue";
import PlaybackSurface from "./components/PlaybackSurface.vue";
import PlayerControls from "./components/PlayerControls.vue";
import { scanMediaRoot } from "../../api/mediaLibrary";
import { describeEngine } from "../../player/mediaTypes";
import { choosePlayback } from "../../player/playbackRouter";
import { useLibraryStore } from "../../store/modules/library";
import type { MediaTreeNode, SelectedMedia } from "../../types/media";

const library = useLibraryStore();
const loading = ref(false);
const playbackMessage = ref("请选择课程目录");

const selectedEngineName = computed(() =>
  library.selectedMedia ? describeEngine(library.selectedMedia.engine) : "未选择"
);

async function chooseFolder() {
  const selected = await open({
    directory: true,
    multiple: false,
    title: "选择课程目录"
  });

  if (!selected || Array.isArray(selected)) return;
  await loadFolder(selected);
}

async function loadFolder(rootPath = library.rootPath) {
  if (!rootPath) return;

  loading.value = true;
  try {
    const nodes = await scanMediaRoot(rootPath);
    library.setRoot(rootPath, nodes);
    playbackMessage.value = nodes.length ? "选择左侧文件开始播放" : "目录中没有发现可播放文件";
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    ElMessage.error(message);
  } finally {
    loading.value = false;
  }
}

function handleSelect(node: MediaTreeNode) {
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
</script>

<template>
  <main class="player-workspace">
    <aside class="library-pane">
      <header class="pane-header">
        <div class="title-row">
          <el-icon><Monitor /></el-icon>
          <span>课程目录</span>
        </div>
        <div class="header-actions">
          <el-tooltip content="刷新目录" placement="bottom">
            <el-button
              circle
              :icon="Refresh"
              :disabled="!library.rootPath || loading"
              @click="loadFolder()"
            />
          </el-tooltip>
          <el-tooltip content="选择目录" placement="bottom">
            <el-button circle type="primary" :icon="FolderOpened" @click="chooseFolder" />
          </el-tooltip>
        </div>
      </header>

      <div class="root-path" :title="library.rootPath">
        {{ library.rootPath || "未选择目录" }}
      </div>

      <FileTree :nodes="library.nodes" :loading="loading" @select="handleSelect" />
    </aside>

    <section class="playback-pane">
      <header class="playback-header">
        <div class="media-title">
          <span>{{ library.selectedMedia?.name || "未选择文件" }}</span>
          <small>{{ selectedEngineName }}</small>
        </div>
        <el-button :icon="VideoPlay" type="primary" :disabled="!library.selectedMedia">
          播放
        </el-button>
      </header>

      <PlaybackSurface :media="library.selectedMedia" :message="playbackMessage" />
      <PlayerControls :disabled="!library.selectedMedia" />
    </section>
  </main>
</template>

<style scoped lang="scss">
.player-workspace {
  display: grid;
  grid-template-columns: minmax(280px, 340px) minmax(0, 1fr);
  width: 100vw;
  height: 100vh;
  background: #f4f6f8;
}

.library-pane {
  display: flex;
  min-width: 0;
  flex-direction: column;
  border-right: 1px solid #d9e0e7;
  background: #ffffff;
}

.pane-header,
.playback-header {
  display: flex;
  min-height: 56px;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 16px;
  border-bottom: 1px solid #d9e0e7;
}

.title-row,
.header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.title-row {
  min-width: 0;
  font-weight: 700;
}

.root-path {
  min-height: 36px;
  overflow: hidden;
  padding: 9px 16px;
  border-bottom: 1px solid #edf1f4;
  color: #697684;
  font-size: 12px;
  line-height: 18px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.playback-pane {
  display: grid;
  min-width: 0;
  grid-template-rows: auto minmax(0, 1fr) auto;
}

.media-title {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 4px;
}

.media-title span {
  overflow: hidden;
  max-width: 62vw;
  font-weight: 700;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.media-title small {
  color: #697684;
  font-size: 12px;
}

@media (max-width: 860px) {
  .player-workspace {
    grid-template-columns: 1fr;
    grid-template-rows: minmax(220px, 38vh) minmax(0, 1fr);
  }

  .library-pane {
    border-right: 0;
    border-bottom: 1px solid #d9e0e7;
  }

  .media-title span {
    max-width: 48vw;
  }
}
</style>
