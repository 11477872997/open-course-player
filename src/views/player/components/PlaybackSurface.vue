<script setup lang="ts">
import type { ComponentPublicInstance } from "vue";
import {
  Calendar,
  Close,
  DArrowLeft,
  DArrowRight,
  FolderAdd,
  FolderOpened,
  FullScreen,
  Headset,
  Monitor,
  Timer,
  VideoPause,
  VideoPlay
} from "@element-plus/icons-vue";
import type { SelectedMedia } from "../../../types/media";

const props = defineProps<{
  media: SelectedMedia | null;
  sourceUrl: string;
  message: string;
  engineName: string;
  playing: boolean;
  currentTime: number;
  duration: number;
  volume: number;
  playbackRate: number;
  fullscreen: boolean;
}>();

const emit = defineEmits<{
  addFolder: [];
  previous: [];
  next: [];
  playPause: [];
  fullscreen: [element: HTMLElement];
  exitFullscreen: [];
  mediaMounted: [element: HTMLMediaElement | null];
  loadedMetadata: [];
  timeUpdate: [];
  durationChange: [];
  mediaError: [];
  seek: [value: number];
  volume: [value: number];
  rate: [value: number];
  play: [];
  pause: [];
  ended: [];
  openLocation: [];
}>();

const playbackRates = [0.75, 1, 1.25, 1.5, 2];
let shellElement: HTMLElement | null = null;

function setMediaRef(element: Element | ComponentPublicInstance | null) {
  emit("mediaMounted", element instanceof HTMLMediaElement ? element : null);
}

function setShellRef(element: Element | ComponentPublicInstance | null) {
  shellElement = element instanceof HTMLElement ? element : null;
}

function formatTime(seconds: number, fallback = "00:00") {
  if (!Number.isFinite(seconds) || seconds < 0) return fallback;
  const total = Math.floor(seconds);
  const minutes = Math.floor(total / 60);
  const rest = total % 60;
  return `${String(minutes).padStart(2, "0")}:${String(rest).padStart(2, "0")}`;
}

function formatDuration(seconds: number) {
  return Number.isFinite(seconds) && seconds > 0 ? formatTime(seconds, "--:--") : "--:--";
}

function seekPercent() {
  if (!Number.isFinite(props.duration) || props.duration <= 0) return 0;
  return Math.min(100, Math.max(0, (props.currentTime / props.duration) * 100));
}

function volumePercent() {
  return Math.min(100, Math.max(0, props.volume * 100));
}

function handleSeek(event: Event) {
  emit("seek", Number((event.target as HTMLInputElement).value));
}

function handleVolume(event: Event) {
  emit("volume", Number((event.target as HTMLInputElement).value));
}

function togglePlaybackRate() {
  const currentIndex = playbackRates.findIndex((rate) => rate === props.playbackRate);
  const nextIndex = currentIndex < 0 ? 1 : (currentIndex + 1) % playbackRates.length;
  emit("rate", playbackRates[nextIndex]);
}

function requestFullscreen() {
  if (shellElement) emit("fullscreen", shellElement);
}
</script>

<template>
  <div class="playback-surface">
    <div :ref="setShellRef" class="video-card" :class="{ fullscreen }">
      <div class="player-stage">
        <video
          v-if="media?.kind === 'video' && sourceUrl"
          :ref="setMediaRef"
          class="video-element"
          :src="media.engine === 'mpegts' || media.engine === 'easy-player' ? undefined : sourceUrl"
          playsinline
          @loadedmetadata="emit('loadedMetadata')"
          @canplay="emit('loadedMetadata')"
          @durationchange="emit('durationChange')"
          @error="emit('mediaError')"
          @timeupdate="emit('timeUpdate')"
          @play="emit('play')"
          @pause="emit('pause')"
          @ended="emit('ended')"
        />

        <div v-else-if="media?.kind === 'audio' && sourceUrl" class="audio-stage">
          <audio
            :ref="setMediaRef"
            :src="sourceUrl"
            preload="metadata"
            @loadedmetadata="emit('loadedMetadata')"
            @canplay="emit('loadedMetadata')"
            @durationchange="emit('durationChange')"
            @error="emit('mediaError')"
            @timeupdate="emit('timeUpdate')"
            @play="emit('play')"
            @pause="emit('pause')"
            @ended="emit('ended')"
          />
          <div class="audio-disc">
            <el-icon :size="54"><Headset /></el-icon>
          </div>
          <strong :title="media.name">{{ media.name }}</strong>
          <span>{{ playing ? "音频正在播放" : "音频已就绪" }}</span>
        </div>

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
          <span>{{ media ? (playing ? "正在播放" : "已就绪") : "等待选择" }}</span>
          <button
            v-if="media?.kind === 'video' && sourceUrl"
            class="stage-tool"
            type="button"
            :title="fullscreen ? '退出全屏' : '视频全屏'"
            @click="fullscreen ? emit('exitFullscreen') : requestFullscreen()"
          >
            <el-icon><Close v-if="fullscreen" /><FullScreen v-else /></el-icon>
          </button>
        </div>

        <div v-if="fullscreen && media && sourceUrl" class="fullscreen-controls">
          <div class="fullscreen-title">
            <strong :title="media.name">{{ media.name }}</strong>
            <span>{{ formatTime(currentTime) }} / {{ formatDuration(duration) }}</span>
          </div>

          <label class="fs-progress" :style="{ '--fill': `${seekPercent()}%` }">
            <input
              type="range"
              min="0"
              :max="duration > 0 ? duration : 0"
              step="0.1"
              :value="currentTime"
              :disabled="duration <= 0"
              @input="handleSeek"
            />
          </label>

          <div class="fs-actions">
            <button type="button" title="上一个文件" @click="emit('previous')">
              <el-icon><DArrowLeft /></el-icon>
            </button>
            <button class="primary" type="button" :title="playing ? '暂停' : '播放'" @click="emit('playPause')">
              <el-icon><VideoPause v-if="playing" /><VideoPlay v-else /></el-icon>
            </button>
            <button type="button" title="下一个文件" @click="emit('next')">
              <el-icon><DArrowRight /></el-icon>
            </button>

            <span class="fs-time">
              <el-icon><Timer /></el-icon>
              {{ formatTime(currentTime) }} / {{ formatDuration(duration) }}
            </span>

            <label class="fs-volume" :style="{ '--fill': `${volumePercent()}%` }">
              <el-icon><Headset /></el-icon>
              <input
                type="range"
                min="0"
                max="1"
                step="0.01"
                :value="volume"
                @input="handleVolume"
              />
            </label>

            <button class="speed" type="button" title="切换倍速" @click="togglePlaybackRate">
              {{ playbackRate.toFixed(playbackRate % 1 === 0 ? 1 : 2) }}x
            </button>
            <button type="button" title="退出全屏" @click="emit('exitFullscreen')">
              <el-icon><Close /></el-icon>
            </button>
          </div>
        </div>
      </div>

      <section class="media-info">
        <div class="file-copy">
          <strong :title="media?.name">{{ media?.name || "未选择文件" }}</strong>
          <span :title="media?.path">{{ media?.path || "从右侧章节列表选择一个可播放文件" }}</span>
        </div>

        <div class="meta-row">
          <span>{{ engineName }}</span>
          <span>{{ sourceUrl ? "本地文件" : "未加载" }}</span>
          <span>{{ formatDuration(duration) }}</span>
          <button
            v-if="media"
            class="meta-action"
            type="button"
            @click="emit('openLocation')"
          >
            <el-icon><FolderOpened /></el-icon>
            打开文件所在位置
          </button>
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

.video-card.fullscreen {
  width: 100vw;
  height: 100vh;
  grid-template-rows: minmax(0, 1fr);
  border: 0;
  border-radius: 0;
  background: #03060b;
}

.video-card.fullscreen .media-info {
  display: none;
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

.audio-stage {
  display: grid;
  max-width: min(520px, 72%);
  justify-items: center;
  gap: 12px;
  color: #d8e5f7;
  text-align: center;
}

.audio-stage audio {
  display: none;
}

.audio-stage strong {
  overflow: hidden;
  max-width: 100%;
  color: #f8fafc;
  font-size: 18px;
  font-weight: 750;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.audio-stage span {
  color: #93a6bd;
  font-size: 12px;
}

.audio-disc {
  display: grid;
  width: 118px;
  height: 118px;
  place-items: center;
  border: 1px solid rgba(148, 163, 184, 0.2);
  border-radius: 999px;
  background:
    radial-gradient(circle at 50% 50%, rgba(59, 130, 246, 0.28) 0 18%, transparent 19%),
    linear-gradient(145deg, rgba(30, 41, 59, 0.94), rgba(15, 23, 42, 0.62));
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.08),
    0 18px 42px rgba(0, 0, 0, 0.34);
  color: #cfe1ff;
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

.video-card.fullscreen .center-play {
  width: 84px;
  height: 84px;
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
  z-index: 3;
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

.stage-tool,
.fs-actions button {
  display: grid;
  place-items: center;
  border: 1px solid rgba(148, 163, 184, 0.14);
  background: rgba(15, 23, 42, 0.58);
  color: #d8e5f7;
  cursor: pointer;
  transition:
    background 0.16s ease,
    border-color 0.16s ease,
    color 0.16s ease;
}

.stage-tool {
  width: 30px;
  height: 30px;
  border-radius: 6px;
  pointer-events: auto;
}

.stage-tool:hover,
.fs-actions button:hover {
  border-color: rgba(148, 163, 184, 0.32);
  background: rgba(30, 41, 59, 0.72);
  color: #ffffff;
}

.fullscreen-controls {
  position: absolute;
  right: 0;
  bottom: 0;
  left: 0;
  z-index: 4;
  display: grid;
  gap: 10px;
  padding: 56px 24px 22px;
  background: linear-gradient(0deg, rgba(0, 0, 0, 0.82), rgba(0, 0, 0, 0));
}

.fullscreen-title {
  display: flex;
  min-width: 0;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  color: #f8fafc;
}

.fullscreen-title strong {
  overflow: hidden;
  font-size: 14px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.fullscreen-title span,
.fs-time {
  color: #d6e3f2;
  font-size: 12px;
  font-variant-numeric: tabular-nums;
}

.fs-progress,
.fs-volume {
  position: relative;
  display: flex;
  align-items: center;
}

.fs-progress::before,
.fs-volume::before {
  position: absolute;
  right: 0;
  left: 0;
  height: 4px;
  border-radius: 999px;
  background:
    linear-gradient(90deg, var(--ocp-primary) var(--fill), transparent var(--fill)),
    rgba(148, 163, 184, 0.28);
  content: "";
}

.fs-progress input,
.fs-volume input {
  position: relative;
  z-index: 1;
  width: 100%;
  height: 18px;
  margin: 0;
  appearance: none;
  background: transparent;
  cursor: pointer;
}

.fs-progress input::-webkit-slider-runnable-track,
.fs-volume input::-webkit-slider-runnable-track {
  height: 4px;
  background: transparent;
}

.fs-progress input::-webkit-slider-thumb,
.fs-volume input::-webkit-slider-thumb {
  width: 12px;
  height: 12px;
  margin-top: -4px;
  appearance: none;
  border: 2px solid #ffffff;
  border-radius: 999px;
  background: var(--ocp-primary);
}

.fs-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

.fs-actions button {
  width: 34px;
  height: 34px;
  border-radius: 999px;
}

.fs-actions button.primary {
  border-color: rgba(59, 130, 246, 0.76);
  background: var(--ocp-primary);
  color: #ffffff;
}

.fs-time {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  margin-left: 4px;
}

.fs-volume {
  width: 120px;
  gap: 8px;
  margin-left: auto;
  color: #d8e5f7;
}

.fs-volume::before {
  left: 24px;
}

.fs-volume input {
  flex: 1;
}

.fs-actions button.speed {
  width: 58px;
  border-radius: 6px;
  font-size: 12px;
  font-variant-numeric: tabular-nums;
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

.meta-action {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  min-height: 22px;
  padding: 0 8px;
  border: 1px solid rgba(59, 130, 246, 0.22);
  border-radius: 5px;
  background: rgba(59, 130, 246, 0.12);
  color: #cfe1ff;
  font-size: 11px;
  cursor: pointer;
  transition:
    background 0.16s ease,
    border-color 0.16s ease,
    color 0.16s ease;
}

.meta-action:hover {
  border-color: rgba(59, 130, 246, 0.44);
  background: rgba(59, 130, 246, 0.2);
  color: #ffffff;
}

@media (max-width: 760px) {
  .fullscreen-controls {
    padding: 52px 14px 16px;
  }

  .fs-actions {
    gap: 7px;
  }

  .fs-volume {
    width: 84px;
  }
}
</style>
