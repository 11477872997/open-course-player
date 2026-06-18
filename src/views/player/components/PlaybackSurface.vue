<script setup lang="ts">
import type { ComponentPublicInstance } from "vue";
import {
  ArrowLeft,
  ArrowRight,
  Calendar,
  FolderAdd,
  FullScreen,
  Monitor,
  VideoPause,
  VideoPlay
} from "@element-plus/icons-vue";
import type { SelectedMedia } from "../../../types/media";

defineProps<{
  media: SelectedMedia | null;
  sourceUrl: string;
  message: string;
  engineName: string;
  playing: boolean;
  duration: number;
}>();

const emit = defineEmits<{
  addFolder: [];
  previous: [];
  next: [];
  playPause: [];
  fullscreen: [];
  videoMounted: [element: HTMLVideoElement | null];
  loadedMetadata: [];
  timeUpdate: [];
  play: [];
  pause: [];
  ended: [];
}>();

function setVideoRef(element: Element | ComponentPublicInstance | null) {
  emit("videoMounted", element instanceof HTMLVideoElement ? element : null);
}

function formatDuration(seconds: number) {
  if (!Number.isFinite(seconds) || seconds <= 0) return "--:--";
  const total = Math.floor(seconds);
  const minutes = Math.floor(total / 60);
  const rest = total % 60;
  return `${String(minutes).padStart(2, "0")}:${String(rest).padStart(2, "0")}`;
}
</script>

<template>
  <div class="playback-surface">
    <div class="video-card">
      <div class="player-stage" @dblclick="emit('fullscreen')">
        <video
          v-if="media && sourceUrl"
          :ref="setVideoRef"
          class="video-element"
          :src="sourceUrl"
          playsinline
          @loadedmetadata="emit('loadedMetadata')"
          @timeupdate="emit('timeUpdate')"
          @play="emit('play')"
          @pause="emit('pause')"
          @ended="emit('ended')"
        />

        <button v-if="media && sourceUrl" class="center-play" type="button" @click="emit('playPause')">
          <el-icon :size="44">
            <VideoPause v-if="playing" />
            <VideoPlay v-else />
          </el-icon>
        </button>

        <div v-else class="empty-stage">
          <div class="empty-icon">
            <el-icon :size="38"><Monitor /></el-icon>
          </div>
          <strong>选择本地视频</strong>
          <span>{{ message }}</span>
          <el-button type="primary" :icon="FolderAdd" @click="emit('addFolder')">
            添加文件夹
          </el-button>
        </div>

        <div class="stage-top">
          <span>{{ media ? "正在播放" : "等待选择" }}</span>
          <div class="stage-tools">
            <button type="button" @click="emit('fullscreen')">
              <el-icon><FullScreen /></el-icon>
            </button>
          </div>
        </div>
      </div>

      <section class="media-info">
        <div class="file-copy">
          <strong :title="media?.name">{{ media?.name || "未选择文件" }}</strong>
          <span :title="media?.path">{{ media?.path || "从右侧章节列表选择一个可播放文件" }}</span>
        </div>

        <div class="file-actions">
          <el-button :icon="ArrowLeft" :disabled="!media" @click="emit('previous')">上一文件</el-button>
          <el-button type="primary" :icon="ArrowRight" :disabled="!media" @click="emit('next')">下一文件</el-button>
        </div>

        <div class="meta-row">
          <span>{{ engineName }}</span>
          <span>{{ sourceUrl ? "本地文件" : "未加载" }}</span>
          <span>{{ formatDuration(duration) }}</span>
          <span><el-icon><Calendar /></el-icon> {{ media ? "已选择" : "等待中" }}</span>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped lang="scss">
.playback-surface {
  display: grid;
  min-height: 0;
}

.video-card {
  display: grid;
  min-width: 0;
  min-height: 0;
  grid-template-rows: minmax(0, 1fr) auto;
  overflow: hidden;
  border: 1px solid rgba(148, 163, 184, 0.18);
  border-radius: 6px;
  background: rgba(16, 23, 34, 0.72);
}

.player-stage {
  position: relative;
  display: grid;
  min-height: 0;
  place-items: center;
  overflow: hidden;
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.04), rgba(255, 255, 255, 0) 28%),
    linear-gradient(0deg, rgba(0, 0, 0, 0.72), rgba(0, 0, 0, 0) 34%),
    #080b10;
}

.video-element {
  width: 100%;
  height: 100%;
  background: #080b10;
  object-fit: contain;
}

.center-play {
  position: absolute;
  display: grid;
  width: 74px;
  height: 74px;
  place-items: center;
  border: 1px solid rgba(255, 255, 255, 0.14);
  border-radius: 999px;
  background: rgba(0, 0, 0, 0.34);
  color: #ffffff;
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.16s ease;
}

.player-stage:hover .center-play {
  opacity: 1;
}

.empty-stage {
  display: flex;
  align-items: center;
  max-width: 340px;
  flex-direction: column;
  gap: 10px;
  padding: 28px;
  color: #cbd5e1;
  text-align: center;
}

.empty-stage strong {
  color: #f8fafc;
  font-size: 16px;
}

.empty-stage span {
  color: #95a7bd;
  font-size: 12px;
  line-height: 18px;
}

.empty-icon {
  display: grid;
  width: 66px;
  height: 66px;
  place-items: center;
  border: 1px solid rgba(148, 163, 184, 0.18);
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.055);
  color: #cfe1ff;
}

.stage-top {
  position: absolute;
  top: 14px;
  right: 14px;
  left: 14px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  pointer-events: none;
}

.stage-top span {
  padding: 5px 9px;
  border-radius: 5px;
  background: rgba(15, 23, 42, 0.66);
  color: #e5edf8;
  font-size: 12px;
}

.stage-tools {
  pointer-events: auto;
}

.stage-tools button {
  display: grid;
  width: 30px;
  height: 30px;
  place-items: center;
  border-radius: 6px;
  background: rgba(15, 23, 42, 0.48);
  color: #d8e5f7;
  cursor: pointer;
}

.media-info {
  display: grid;
  min-width: 0;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 10px 14px;
  padding: 12px;
  border-top: 1px solid rgba(148, 163, 184, 0.14);
}

.file-copy {
  min-width: 0;
}

.file-copy strong {
  overflow: hidden;
  display: block;
  color: var(--ocp-text-inverse);
  font-size: 14px;
  font-weight: 700;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-copy span {
  overflow: hidden;
  display: block;
  margin-top: 5px;
  color: var(--ocp-text-inverse-muted);
  font-size: 11px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.file-actions :deep(.el-button) {
  min-height: 30px;
  padding: 0 10px;
}

.meta-row {
  display: flex;
  min-width: 0;
  grid-column: 1 / -1;
  flex-wrap: wrap;
  gap: 8px;
}

.meta-row span {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  min-height: 22px;
  padding: 0 7px;
  border-radius: 5px;
  background: rgba(255, 255, 255, 0.045);
  color: #a6b6ca;
  font-size: 11px;
}
</style>
