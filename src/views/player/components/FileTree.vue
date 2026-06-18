<script setup lang="ts">
import { computed } from "vue";
import { ArrowRight, CircleCheck, VideoPlay } from "@element-plus/icons-vue";
import type { MediaTreeNode } from "../../../types/media";

const props = defineProps<{
  nodes: MediaTreeNode[];
  loading: boolean;
  activeMediaId: string;
}>();

const emit = defineEmits<{
  select: [node: MediaTreeNode];
}>();

const hasNodes = computed(() => props.nodes.length > 0);

function timeFor(index: number) {
  const minutes = 12 + (index % 7);
  const seconds = String((index * 5) % 60).padStart(2, "0");
  return `${minutes}:${seconds}`;
}
</script>

<template>
  <div class="chapter-list">
    <el-skeleton v-if="loading" :rows="10" animated />

    <el-empty v-else-if="!hasNodes" description="当前资料库没有文件" :image-size="72" />

    <el-tree
      v-else
      node-key="id"
      :data="nodes"
      :props="{ label: 'name', children: 'children' }"
      :expand-on-click-node="false"
      default-expand-all
      @node-click="emit('select', $event)"
    >
      <template #default="{ data, node }">
        <div
          class="chapter-row"
          :class="{
            active: data.id === activeMediaId,
            folder: data.kind === 'folder',
            disabled: !data.playable && data.kind !== 'folder'
          }"
        >
          <template v-if="data.kind === 'folder'">
            <el-icon class="folder-arrow"><ArrowRight /></el-icon>
            <span class="chapter-name" :title="data.name">{{ data.name }}</span>
            <em>{{ data.children?.length || 0 }}</em>
          </template>

          <template v-else>
            <span class="timeline-dot">
              <el-icon v-if="data.id === activeMediaId"><VideoPlay /></el-icon>
            </span>
            <span class="chapter-name" :title="data.name">{{ node.level > 1 ? "" : "03-" }}{{ data.name }}</span>
            <time>{{ timeFor(node.level + data.name.length) }}</time>
            <el-icon class="done"><CircleCheck /></el-icon>
          </template>
        </div>
      </template>
    </el-tree>
  </div>
</template>

<style scoped lang="scss">
.chapter-list {
  min-height: 0;
  overflow: auto;
  padding: 8px;
}

:deep(.el-tree) {
  --el-tree-node-hover-bg-color: transparent;
  background: transparent;
  color: inherit;
}

:deep(.el-tree-node__content) {
  height: 30px;
  padding-right: 0;
  border-radius: 5px;
}

:deep(.el-tree-node__expand-icon) {
  color: #7890aa;
}

.chapter-row {
  display: grid;
  width: 100%;
  min-width: 0;
  grid-template-columns: 18px minmax(0, 1fr) auto 18px;
  align-items: center;
  gap: 7px;
  min-height: 28px;
  padding: 0 7px;
  border-radius: 5px;
  color: #9eb0c6;
}

.chapter-row:hover {
  background: rgba(255, 255, 255, 0.05);
  color: #e7eef8;
}

.chapter-row.active {
  background: rgba(47, 129, 247, 0.16);
  color: #e7f0ff;
}

.chapter-row.folder {
  grid-template-columns: 18px minmax(0, 1fr) auto;
  color: #c7d4e5;
  font-weight: 650;
}

.chapter-row.disabled {
  color: #66788e;
}

.folder-arrow {
  color: #8aa0ba;
}

.chapter-name {
  overflow: hidden;
  font-size: 12px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.chapter-row em,
.chapter-row time {
  color: #8296ad;
  font-size: 11px;
  font-style: normal;
  font-variant-numeric: tabular-nums;
}

.timeline-dot {
  position: relative;
  display: grid;
  width: 16px;
  height: 16px;
  place-items: center;
  border: 1px solid rgba(59, 130, 246, 0.5);
  border-radius: 999px;
}

.timeline-dot::before {
  content: "";
  width: 5px;
  height: 5px;
  border-radius: 999px;
  background: #3b82f6;
}

.chapter-row.active .timeline-dot {
  border-color: var(--ocp-primary);
  background: var(--ocp-primary);
  color: #ffffff;
}

.chapter-row.active .timeline-dot::before {
  display: none;
}

.done {
  color: #61758d;
}

.chapter-row.active .done {
  color: var(--ocp-primary);
}
</style>
