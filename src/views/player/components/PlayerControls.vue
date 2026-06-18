<script setup lang="ts">
import {
  FullScreen,
  Headset,
  VideoPause,
  VideoPlay,
  DArrowLeft,
  DArrowRight
} from "@element-plus/icons-vue";

defineProps<{
  disabled: boolean;
}>();
</script>

<template>
  <footer class="player-controls">
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

    <el-slider class="progress" :model-value="0" :disabled="disabled" />

    <div class="side-controls">
      <el-icon :size="18"><Headset /></el-icon>
      <el-slider class="volume" :model-value="80" :disabled="disabled" />
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
  min-height: 72px;
  grid-template-columns: auto minmax(140px, 1fr) auto;
  align-items: center;
  gap: 16px;
  padding: 12px 16px;
  border-top: 1px solid #d9e0e7;
  background: #ffffff;
}

.transport,
.side-controls {
  display: flex;
  align-items: center;
  gap: 8px;
}

.progress {
  min-width: 140px;
}

.volume {
  width: 92px;
}

.speed {
  width: 88px;
}

@media (max-width: 860px) {
  .player-controls {
    grid-template-columns: 1fr;
    gap: 8px;
  }

  .transport,
  .side-controls {
    justify-content: center;
  }
}
</style>
