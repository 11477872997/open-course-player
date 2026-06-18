<script setup lang="ts">
import { Collection, Delete, FolderAdd, Refresh, VideoCamera } from "@element-plus/icons-vue";
import type { MediaLibraryRoot } from "../../../types/media";

defineProps<{
  roots: MediaLibraryRoot[];
  activeRootId: string;
  loading: boolean;
  totalPlayableFiles: number;
}>();

const emit = defineEmits<{
  add: [];
  refresh: [root: MediaLibraryRoot];
  select: [root: MediaLibraryRoot];
  remove: [root: MediaLibraryRoot];
}>();

function pathTail(path: string) {
  const parts = path.split(/[\\\/]/).filter(Boolean);
  return parts.slice(-2).join(" / ") || path;
}
</script>

<template>
  <aside class="library-list">
    <header class="app-brand">
      <img src="../../../assets/brand/cd-player.png" alt="" />
      <div>
        <strong>Open Course Player</strong>
        <span>本地课程播放器</span>
      </div>
    </header>

    <section class="library-summary">
      <div>
        <span>资料库</span>
        <strong>{{ roots.length }}</strong>
      </div>
      <div>
        <span>可播放</span>
        <strong>{{ totalPlayableFiles }}</strong>
      </div>
    </section>

    <div class="library-actions">
      <el-button type="primary" :icon="FolderAdd" @click="emit('add')">添加文件夹</el-button>
      <el-tooltip content="刷新当前资料库" placement="bottom">
        <el-button
          :icon="Refresh"
          :disabled="!activeRootId || loading"
          @click="roots.find((root) => root.id === activeRootId) && emit('refresh', roots.find((root) => root.id === activeRootId)!)"
        />
      </el-tooltip>
    </div>

    <div class="section-title">
      <el-icon><Collection /></el-icon>
      <span>课程文件夹</span>
    </div>

    <div v-if="roots.length" class="root-list">
      <button
        v-for="root in roots"
        :key="root.id"
        class="root-item"
        :class="{ active: root.id === activeRootId }"
        type="button"
        @click="emit('select', root)"
      >
        <span class="active-mark" />
        <span class="root-icon">
          <el-icon><VideoCamera /></el-icon>
        </span>
        <span class="root-main">
          <strong :title="root.name">{{ root.name }}</strong>
          <small :title="root.path">{{ pathTail(root.path) }}</small>
          <em>{{ root.playableFiles }} 个可播放</em>
        </span>
        <el-tooltip content="移除资料库" placement="right">
          <span
            class="remove"
            role="button"
            tabindex="0"
            @click.stop="emit('remove', root)"
            @keydown.enter.stop="emit('remove', root)"
          >
            <el-icon><Delete /></el-icon>
          </span>
        </el-tooltip>
      </button>
    </div>

    <div v-else class="empty-library">
      <span>还没有资料库</span>
      <small>添加一个或多个课程文件夹后开始浏览</small>
    </div>
  </aside>
</template>

<style scoped lang="scss">
.library-list {
  display: grid;
  min-width: 0;
  grid-template-rows: auto auto auto auto minmax(0, 1fr);
  gap: 14px;
  padding: 14px;
  border-right: 1px solid var(--ocp-border);
  background: #fbfcfe;
}

.app-brand {
  display: grid;
  grid-template-columns: 40px minmax(0, 1fr);
  align-items: center;
  gap: 10px;
  min-height: 44px;
}

.app-brand img {
  width: 40px;
  height: 40px;
}

.app-brand strong,
.root-main strong {
  overflow: hidden;
  display: block;
  color: var(--ocp-text);
  font-size: 14px;
  font-weight: 700;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.app-brand span,
.root-main small,
.root-main em,
.empty-library small {
  overflow: hidden;
  display: block;
  color: var(--ocp-text-muted);
  font-size: 12px;
  font-style: normal;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.library-summary {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}

.library-summary div {
  min-width: 0;
  padding: 10px;
  border: 1px solid var(--ocp-border);
  border-radius: 8px;
  background: var(--ocp-surface);
}

.library-summary span {
  display: block;
  color: var(--ocp-text-muted);
  font-size: 12px;
}

.library-summary strong {
  display: block;
  margin-top: 4px;
  color: var(--ocp-text);
  font-size: 18px;
  font-weight: 700;
}

.library-actions {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 40px;
  gap: 8px;
}

.section-title {
  display: flex;
  align-items: center;
  gap: 6px;
  color: #405063;
  font-size: 12px;
  font-weight: 700;
}

.root-list {
  min-height: 0;
  overflow: auto;
  padding-right: 2px;
}

.root-item {
  position: relative;
  display: grid;
  width: 100%;
  min-width: 0;
  grid-template-columns: 28px minmax(0, 1fr) 28px;
  align-items: center;
  gap: 9px;
  margin-bottom: 8px;
  padding: 10px 8px 10px 10px;
  border: 1px solid transparent;
  border-radius: 8px;
  background: transparent;
  color: inherit;
  cursor: pointer;
  text-align: left;
}

.root-item:hover {
  background: #f1f5f9;
}

.root-item.active {
  border-color: #bdd1ff;
  background: var(--ocp-primary-soft);
}

.active-mark {
  position: absolute;
  inset: 10px auto 10px 0;
  width: 3px;
  border-radius: 999px;
  background: transparent;
}

.root-item.active .active-mark {
  background: var(--ocp-primary);
}

.root-icon {
  display: grid;
  width: 28px;
  height: 28px;
  place-items: center;
  border-radius: 6px;
  background: #eef3f8;
  color: #4b6078;
}

.root-item.active .root-icon {
  background: #dce8ff;
  color: var(--ocp-primary);
}

.remove {
  display: grid;
  width: 26px;
  height: 26px;
  place-items: center;
  border-radius: 6px;
  color: #8b98a7;
}

.remove:hover {
  background: #fee2e2;
  color: var(--ocp-danger);
}

.empty-library {
  display: flex;
  min-height: 180px;
  align-items: center;
  justify-content: center;
  flex-direction: column;
  gap: 6px;
  padding: 20px;
  border: 1px dashed var(--ocp-border);
  border-radius: 8px;
  color: var(--ocp-text);
  text-align: center;
}
</style>
