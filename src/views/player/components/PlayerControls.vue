<script setup lang="ts">
import { computed } from "vue";
import {
  DArrowLeft,
  DArrowRight,
  Headset,
  Timer,
  VideoPause,
  VideoPlay
} from "@element-plus/icons-vue";

const props = defineProps<{
  disabled: boolean;
  playing: boolean;
  currentTime: number;
  duration: number;
  volume: number;
  playbackRate: number;
}>();

const emit = defineEmits<{
  playPause: [];
  previous: [];
  next: [];
  seek: [value: number];
  volume: [value: number];
  rate: [value: number];
}>();

const playbackRates = [0.75, 1, 1.25, 1.5, 2];

const hasDuration = computed(() => Number.isFinite(props.duration) && props.duration > 0);
const progressPercent = computed(() => {
  if (!hasDuration.value) return 0;
  return Math.min(100, Math.max(0, (props.currentTime / props.duration) * 100));
});
const volumePercent = computed(() => Math.min(100, Math.max(0, props.volume * 100)));

function formatTime(seconds: number, fallback = "00:00") {
  if (!Number.isFinite(seconds) || seconds < 0) return fallback;
  const total = Math.floor(seconds);
  const minutes = Math.floor(total / 60);
  const rest = total % 60;
  return `${String(minutes).padStart(2, "0")}:${String(rest).padStart(2, "0")}`;
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
</script>

<template>
  <footer class="player-controls" :class="{ 'is-disabled': disabled }">
    <div class="time-readout" aria-label="播放时间">
      <el-icon><Timer /></el-icon>
      <span>{{ formatTime(currentTime) }}</span>
      <i>/</i>
      <span>{{ hasDuration ? formatTime(duration) : "--:--" }}</span>
    </div>

    <div class="transport" aria-label="播放控制">
      <button
        class="control-button"
        type="button"
        title="上一个文件"
        :disabled="disabled"
        @click="emit('previous')"
      >
        <el-icon><DArrowLeft /></el-icon>
      </button>

      <button
        class="control-button primary"
        type="button"
        :title="playing ? '暂停' : '播放'"
        :disabled="disabled"
        @click="emit('playPause')"
      >
        <el-icon>
          <VideoPause v-if="playing" />
          <VideoPlay v-else />
        </el-icon>
      </button>

      <button
        class="control-button"
        type="button"
        title="下一个文件"
        :disabled="disabled"
        @click="emit('next')"
      >
        <el-icon><DArrowRight /></el-icon>
      </button>
    </div>

    <label
      class="range-wrap progress-wrap"
      :style="{ '--fill': `${progressPercent}%` }"
      aria-label="播放进度"
    >
      <input
        type="range"
        min="0"
        :max="hasDuration ? duration : 0"
        step="0.1"
        :value="currentTime"
        :disabled="disabled || !hasDuration"
        @input="handleSeek"
      />
    </label>

    <div class="side-controls" aria-label="声音和倍速">
      <el-icon class="volume-icon"><Headset /></el-icon>
      <label
        class="range-wrap volume-wrap"
        :style="{ '--fill': `${volumePercent}%` }"
        aria-label="音量"
      >
        <input
          type="range"
          min="0"
          max="1"
          step="0.01"
          :value="volume"
          :disabled="disabled"
          @input="handleVolume"
        />
      </label>
      <button
        class="speed-button"
        type="button"
        title="切换倍速"
        :disabled="disabled"
        @click="togglePlaybackRate"
      >
        {{ playbackRate.toFixed(playbackRate % 1 === 0 ? 1 : 2) }}x
      </button>
    </div>
  </footer>
</template>

<style scoped lang="scss">
.player-controls {
  display: grid;
  min-height: 44px;
  grid-template-columns: auto auto minmax(120px, 1fr) auto;
  align-items: center;
  gap: 10px;
  padding: 8px 0 0;
  color: var(--ocp-text-inverse-muted);
}

.time-readout,
.transport,
.side-controls {
  display: flex;
  align-items: center;
}

.time-readout {
  min-width: 88px;
  gap: 4px;
  font-size: 11px;
  font-variant-numeric: tabular-nums;
}

.time-readout i {
  color: rgba(146, 165, 189, 0.58);
  font-style: normal;
}

.transport {
  gap: 8px;
}

.control-button,
.speed-button {
  display: inline-grid;
  align-items: center;
  justify-content: center;
  border: 1px solid rgba(148, 163, 184, 0.18);
  background: rgba(255, 255, 255, 0.055);
  color: #d8e5f7;
  cursor: pointer;
  transition:
    background 0.16s ease,
    border-color 0.16s ease,
    color 0.16s ease,
    transform 0.16s ease;
}

.control-button {
  width: 28px;
  height: 28px;
  border-radius: 999px;
}

.control-button.primary {
  border-color: rgba(59, 130, 246, 0.72);
  background: var(--ocp-primary);
  color: #ffffff;
  box-shadow: 0 6px 18px rgba(59, 130, 246, 0.26);
}

.control-button:hover,
.speed-button:hover {
  border-color: rgba(148, 163, 184, 0.34);
  background: rgba(255, 255, 255, 0.1);
  color: #ffffff;
}

.control-button.primary:hover {
  border-color: rgba(96, 165, 250, 0.86);
  background: var(--ocp-primary-hover);
}

.control-button:active,
.speed-button:active {
  transform: translateY(1px);
}

.control-button:disabled,
.speed-button:disabled,
.range-wrap input:disabled {
  cursor: not-allowed;
  opacity: 0.46;
}

.range-wrap {
  position: relative;
  display: flex;
  align-items: center;
  height: 20px;
}

.range-wrap::before {
  position: absolute;
  right: 0;
  left: 0;
  height: 4px;
  border-radius: 999px;
  background:
    linear-gradient(90deg, var(--ocp-primary) var(--fill), transparent var(--fill)),
    rgba(148, 163, 184, 0.22);
  content: "";
}

.range-wrap input {
  position: relative;
  z-index: 1;
  width: 100%;
  height: 20px;
  margin: 0;
  appearance: none;
  background: transparent;
  cursor: pointer;
}

.range-wrap input::-webkit-slider-runnable-track {
  height: 4px;
  border-radius: 999px;
  background: transparent;
}

.range-wrap input::-webkit-slider-thumb {
  width: 10px;
  height: 10px;
  margin-top: -3px;
  appearance: none;
  border: 2px solid #ffffff;
  border-radius: 999px;
  background: var(--ocp-primary);
  box-shadow: 0 0 0 4px rgba(59, 130, 246, 0.14);
}

.range-wrap input::-moz-range-track {
  height: 4px;
  border-radius: 999px;
  background: transparent;
}

.range-wrap input::-moz-range-thumb {
  width: 10px;
  height: 10px;
  border: 2px solid #ffffff;
  border-radius: 999px;
  background: var(--ocp-primary);
  box-shadow: 0 0 0 4px rgba(59, 130, 246, 0.14);
}

.progress-wrap {
  min-width: 120px;
}

.side-controls {
  gap: 8px;
}

.volume-icon {
  color: #a9bbd0;
}

.volume-wrap {
  width: 78px;
}

.speed-button {
  min-width: 58px;
  height: 28px;
  border-radius: 6px;
  font-size: 11px;
  font-variant-numeric: tabular-nums;
}

@media (max-width: 1080px) {
  .player-controls {
    grid-template-columns: auto auto minmax(90px, 1fr) auto;
    gap: 8px;
  }

  .volume-wrap {
    width: 58px;
  }

  .speed-button {
    min-width: 50px;
  }
}
</style>
