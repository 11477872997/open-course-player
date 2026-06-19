<script setup lang="ts">
import {
  Collection,
  Delete,
  FolderAdd,
  Refresh
} from "@element-plus/icons-vue";
import type { MediaLibraryRoot } from "../../../types/media";

const props = defineProps<{
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
  contextMenu: [root: MediaLibraryRoot, event: MouseEvent];
}>();

function refreshActiveRoot() {
  const root = props.roots.find((item) => item.id === props.activeRootId);
  if (root) emit("refresh", root);
}
</script>

<template>
  <aside class="library-list">
    <div class="sidebar-section">
      <div class="section-title">
        <span>资料库</span>
        <el-tooltip content="添加文件夹" placement="right">
          <button class="plain-icon" type="button" @click="emit('add')">+</button>
        </el-tooltip>
      </div>

      <button
        v-for="root in roots"
        :key="root.id"
        class="nav-item root"
        :class="{ selected: root.id === activeRootId }"
        type="button"
        @click="emit('select', root)"
        @contextmenu.prevent.stop="emit('contextMenu', root, $event)"
      >
        <el-icon><Collection /></el-icon>
        <span :title="root.path">{{ root.name }}</span>
        <em>{{ root.playableFiles }}</em>
        <el-tooltip content="移除" placement="right">
          <i
            class="remove"
            role="button"
            tabindex="0"
            @click.stop="emit('remove', root)"
            @keydown.enter.stop="emit('remove', root)"
          >
            <el-icon><Delete /></el-icon>
          </i>
        </el-tooltip>
      </button>

      <button v-if="roots.length" class="nav-item compact" type="button" @click="refreshActiveRoot">
        <el-icon><Refresh /></el-icon>
        <span>刷新当前</span>
        <em>{{ loading ? "..." : "" }}</em>
      </button>

      <div v-if="!roots.length" class="empty-library">
        <el-icon><FolderAdd /></el-icon>
        <strong>暂无资料库</strong>
        <span>点击右上角 + 添加本地文件夹</span>
      </div>
    </div>

    <div class="brand-placeholder">
      <img src="../../../assets/brand/cd-player.png" alt="" />
      <strong>Open Course Player</strong>
      <span>本地文件播放</span>
    </div>

    <button class="add-folder" type="button" @click="emit('add')">
      <el-icon><FolderAdd /></el-icon>
      <span>添加文件夹</span>
    </button>
  </aside>
</template>

<style scoped lang="scss">
.library-list {
  display: grid;
  min-width: 0;
  min-height: 0;
  grid-template-rows: minmax(0, 1fr) auto;
  gap: 12px;
  padding: 14px 10px;
  border-right: 1px solid var(--ocp-dark-border);
  background: rgba(10, 16, 24, 0.62);
}

.sidebar-section {
  display: grid;
  align-content: start;
  gap: 6px;
  min-width: 0;
  min-height: 0;
  overflow: auto;
}

.section-title {
  display: flex;
  align-items: center;
  justify-content: space-between;
  min-height: 26px;
  padding: 0 4px;
  color: var(--ocp-text-inverse-muted);
  font-size: 12px;
}

.plain-icon {
  display: grid;
  width: 22px;
  height: 22px;
  place-items: center;
  border: 0;
  border-radius: 5px;
  background: transparent;
  color: var(--ocp-text-inverse-muted);
  cursor: pointer;
}

.plain-icon:hover {
  background: rgba(255, 255, 255, 0.08);
  color: var(--ocp-text-inverse);
}

.nav-item {
  position: relative;
  display: grid;
  width: 100%;
  min-width: 0;
  grid-template-columns: 18px minmax(0, 1fr) auto;
  align-items: center;
  gap: 8px;
  min-height: 32px;
  padding: 0 9px;
  border: 1px solid transparent;
  border-radius: 6px;
  background: transparent;
  color: #a6b4c7;
  cursor: pointer;
  text-align: left;
}

.nav-item span {
  overflow: hidden;
  font-size: 12px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.nav-item em {
  color: #8ba0b8;
  font-size: 12px;
  font-style: normal;
}

.nav-item:hover,
.nav-item.selected {
  background: rgba(255, 255, 255, 0.055);
  color: var(--ocp-text-inverse);
}

.nav-item.compact {
  color: #7f92aa;
}

.nav-item.muted {
  color: #98a7ba;
}

.nav-item.root {
  grid-template-columns: 18px minmax(0, 1fr) auto 18px;
}

.remove {
  display: none;
  width: 18px;
  height: 18px;
  place-items: center;
  border-radius: 4px;
  color: #7d90a7;
}

.nav-item.root:hover .remove {
  display: grid;
}

.remove:hover {
  background: rgba(239, 68, 68, 0.14);
  color: #fecaca;
}

.empty-library {
  display: grid;
  justify-items: center;
  gap: 7px;
  margin-top: 8px;
  padding: 18px 10px;
  border: 1px dashed rgba(148, 163, 184, 0.18);
  border-radius: 7px;
  color: var(--ocp-text-inverse-muted);
  text-align: center;
}

.empty-library .el-icon {
  font-size: 24px;
}

.empty-library strong {
  color: var(--ocp-text-inverse);
  font-size: 12px;
}

.empty-library span {
  color: var(--ocp-text-inverse-muted);
  font-size: 11px;
  line-height: 16px;
}

.brand-placeholder {
  display: grid;
  align-self: end;
  min-width: 0;
  justify-items: center;
  gap: 5px;
  padding: 14px 10px;
  border: 1px solid rgba(148, 163, 184, 0.14);
  border-radius: 7px;
  background:
    linear-gradient(180deg, rgba(59, 130, 246, 0.07), rgba(255, 255, 255, 0.025)),
    rgba(255, 255, 255, 0.035);
  text-align: center;
}

.brand-placeholder img {
  width: 42px;
  height: 42px;
  opacity: 0.95;
}

.brand-placeholder strong {
  max-width: 100%;
  overflow: hidden;
  color: var(--ocp-text-inverse);
  font-size: 12px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.brand-placeholder span {
  color: var(--ocp-text-inverse-muted);
  font-size: 11px;
}

.add-folder {
  display: none;
}
</style>
