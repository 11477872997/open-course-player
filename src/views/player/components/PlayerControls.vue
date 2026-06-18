<script setup lang="ts">
import {
  DArrowLeft,
  DArrowRight,
  FullScreen,
  Headset,
  Timer,
  VideoPause,
  VideoPlay
} from "@element-plus/icons-vue";

defineProps<{
  disabled: boolean;
}>();
</script>

<template>
  <footer class="player-controls">
    <div class="time-readout">
      <el-icon><Timer /></el-icon>
      <span>08:24 / 45:30</span>
    </div>

    <div class="transport">
      <el-tooltip content="后退" placement="top">
        <el-button circle :icon="DArrowLeft" :disabled="disabled" />
      </el-tooltip>
      <el-tooltip content="播放" placement="top">
        <el-button circle type="primary" :icon="VideoPlay" :disabled="disabled" />
      </el-tooltip>
      <el-tooltip content="暂停" placement="top">
        <el-button circle :icon="VideoPause" :disabled="disabled" />
      </el-tooltip>
      <el-tooltip content="前进" placement="top">
        <el-button circle :icon="DArrowRight" :disabled="disabled" />
      </el-tooltip>
    </div>

    <el-slider class="progress" :model-value="32" :disabled="disabled" />

    <div class="side-controls">
      <el-icon :size="16"><Headset /></el-icon>
      <el-select model-value="1" class="speed" :disabled="disabled" size="small">
        <el-option label="0.75x" value="0.75" />
        <el-option label="1.0x" value="1" />
        <el-option label="1.25x" value="1.25" />
        <el-option label="1.5x" value="1.5" />
        <el-option label="2.0x" value="2" />
      </el-select>
      <el-tooltip content="全屏" placement="top">
        <el-button circle :icon="FullScreen" :disabled="disabled" />
      </el-tooltip>
    </div>
  </footer>
</template>

<style scoped lang="scss">
.player-controls {
  display: grid;
  min-height: 42px;
  grid-template-columns: auto auto minmax(120px, 1fr) auto;
  align-items: center;
  gap: 10px;
  padding: 8px 2px 0;
  background: transparent;
}

.time-readout,
.transport,
.side-controls {
  display: flex;
  align-items: center;
  gap: 7px;
}

.time-readout,
.side-controls {
  color: var(--ocp-text-inverse-muted);
}

.time-readout {
  font-size: 11px;
  font-variant-numeric: tabular-nums;
}

.transport :deep(.el-button),
.side-controls :deep(.el-button) {
  width: 28px;
  height: 28px;
  min-height: 28px;
  --el-button-bg-color: rgba(255, 255, 255, 0.055);
  --el-button-border-color: rgba(148, 163, 184, 0.18);
  --el-button-text-color: #d8e5f7;
  --el-button-hover-bg-color: rgba(255, 255, 255, 0.1);
  --el-button-hover-border-color: rgba(148, 163, 184, 0.34);
  --el-button-hover-text-color: #ffffff;
}

.transport :deep(.el-button--primary) {
  --el-button-bg-color: var(--ocp-primary);
  --el-button-border-color: var(--ocp-primary);
  --el-button-hover-bg-color: var(--ocp-primary-hover);
  --el-button-hover-border-color: var(--ocp-primary-hover);
}

.progress {
  min-width: 120px;
}

.progress :deep(.el-slider__runway) {
  height: 3px;
  background-color: rgba(148, 163, 184, 0.24);
}

.progress :deep(.el-slider__button) {
  width: 10px;
  height: 10px;
}

.speed {
  width: 58px;
}

.speed :deep(.el-select__wrapper) {
  min-height: 28px;
  padding: 0 6px;
  background: rgba(255, 255, 255, 0.055);
  box-shadow: 0 0 0 1px rgba(148, 163, 184, 0.18) inset;
}

.speed :deep(.el-select__selected-item) {
  color: #d8e5f7;
  font-size: 11px;
}
</style>
