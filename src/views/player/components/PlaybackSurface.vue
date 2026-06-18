<script setup lang="ts">
import { Monitor } from "@element-plus/icons-vue";
import type { SelectedMedia } from "../../../types/media";

defineProps<{
  media: SelectedMedia | null;
  message: string;
}>();
</script>

<template>
  <div class="playback-surface">
    <video v-if="media" class="video-element" controls />
    <div v-else class="empty-state">
      <el-icon :size="44"><Monitor /></el-icon>
      <span>{{ message }}</span>
    </div>
  </div>
</template>

<style scoped lang="scss">
.playback-surface {
  display: grid;
  min-height: 0;
  place-items: center;
  background: #111820;
}

.video-element {
  width: 100%;
  height: 100%;
  background: #111820;
}

.empty-state {
  display: flex;
  align-items: center;
  flex-direction: column;
  gap: 12px;
  color: #c8d2dc;
  font-size: 14px;
}
</style>
