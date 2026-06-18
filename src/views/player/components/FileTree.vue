<script setup lang="ts">
import { computed } from "vue";
import { Document, Files, Film, Folder, Headset } from "@element-plus/icons-vue";
import { describeEngine } from "../../../player/mediaTypes";
import type { MediaTreeNode } from "../../../types/media";

const props = defineProps<{
  nodes: MediaTreeNode[];
  loading: boolean;
}>();

const emit = defineEmits<{
  select: [node: MediaTreeNode];
}>();

const hasNodes = computed(() => props.nodes.length > 0);

function iconFor(node: MediaTreeNode) {
  if (node.kind === "folder") return Folder;
  if (node.kind === "audio") return Headset;
  if (node.kind === "subtitle") return Document;
  if (node.kind === "video") return Film;
  return Files;
}
</script>

<template>
  <div class="file-tree">
    <el-skeleton v-if="loading" :rows="8" animated />

    <el-empty v-else-if="!hasNodes" description="选择目录后显示文件" :image-size="92" />

    <el-tree
      v-else
      node-key="id"
      :data="nodes"
      :props="{ label: 'name', children: 'children' }"
      :expand-on-click-node="false"
      default-expand-all
      @node-click="emit('select', $event)"
    >
      <template #default="{ data }">
        <div class="tree-row" :class="{ disabled: !data.playable && data.kind !== 'folder' }">
          <el-icon><component :is="iconFor(data)" /></el-icon>
          <span class="name" :title="data.name">{{ data.name }}</span>
          <span v-if="data.kind !== 'folder'" class="engine">{{ describeEngine(data.engine) }}</span>
        </div>
      </template>
    </el-tree>
  </div>
</template>

<style scoped lang="scss">
.file-tree {
  min-height: 0;
  flex: 1;
  overflow: auto;
  padding: 10px;
}

.tree-row {
  display: grid;
  width: 100%;
  min-width: 0;
  grid-template-columns: 18px minmax(0, 1fr) auto;
  align-items: center;
  gap: 6px;
  color: #27313c;
}

.tree-row.disabled {
  color: #9aa6b2;
}

.name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.engine {
  min-width: 42px;
  border-radius: 4px;
  background: #eef3f7;
  color: #5d6b78;
  font-size: 11px;
  line-height: 18px;
  text-align: center;
}
</style>
