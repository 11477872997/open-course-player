<script setup lang="ts">
import { computed, defineAsyncComponent } from "vue";
import "@vue-office/docx/lib/v3/index.css";
import "@vue-office/excel/lib/v3/index.css";
import { Document, FolderOpened } from "@element-plus/icons-vue";
import type { SelectedMedia } from "../../../types/media";

const VueOfficeDocx = defineAsyncComponent(() => import("@vue-office/docx/lib/v3/index.js"));
const VueOfficeExcel = defineAsyncComponent(() => import("@vue-office/excel/lib/v3/index.js"));
const VueOfficePdf = defineAsyncComponent(() => import("@vue-office/pdf/lib/v3/index.js"));
const VueOfficePptx = defineAsyncComponent(() => import("@vue-office/pptx/lib/v3/index.js"));

const props = defineProps<{
  media: SelectedMedia | null;
  sourceUrl: string;
  message: string;
}>();

const emit = defineEmits<{
  openLocation: [];
  addFolder: [];
}>();

const extension = computed(() => {
  const name = props.media?.name || "";
  const index = name.lastIndexOf(".");
  return index >= 0 ? name.slice(index + 1).toLowerCase() : "";
});

const previewType = computed(() => {
  switch (extension.value) {
    case "pdf":
      return "pdf";
    case "docx":
      return "docx";
    case "xlsx":
    case "xls":
      return "excel";
    case "pptx":
      return "pptx";
    default:
      return "unsupported";
  }
});

const typeName = computed(() => {
  switch (previewType.value) {
    case "pdf":
      return "PDF";
    case "docx":
      return "Word";
    case "excel":
      return "Excel";
    case "pptx":
      return "PowerPoint";
    default:
      return "文档";
  }
});
</script>

<template>
  <section class="document-preview">
    <header class="document-header">
      <div class="document-title">
        <el-icon><Document /></el-icon>
        <div>
          <strong :title="media?.name">{{ media?.name || "未选择文档" }}</strong>
          <span>{{ sourceUrl ? `${typeName} 预览` : message }}</span>
        </div>
      </div>

      <button v-if="media" class="document-action" type="button" @click="emit('openLocation')">
        <el-icon><FolderOpened /></el-icon>
        打开文件所在位置
      </button>
    </header>

    <div v-if="sourceUrl && previewType !== 'unsupported'" class="document-body">
      <div class="document-viewer" :class="`${previewType}-viewer`">
        <VueOfficePdf
          v-if="previewType === 'pdf'"
          :key="sourceUrl"
          class="office-viewer"
          :src="sourceUrl"
        />
        <VueOfficeDocx
          v-else-if="previewType === 'docx'"
          :key="sourceUrl"
          class="office-viewer"
          :src="sourceUrl"
        />
        <VueOfficeExcel
          v-else-if="previewType === 'excel'"
          :key="sourceUrl"
          class="office-viewer"
          :src="sourceUrl"
        />
        <VueOfficePptx
          v-else-if="previewType === 'pptx'"
          :key="sourceUrl"
          class="office-viewer"
          :src="sourceUrl"
        />
      </div>
    </div>

    <div v-else class="document-empty">
      <div class="empty-icon">
        <el-icon :size="38"><Document /></el-icon>
      </div>
      <strong>{{ media ? "当前文档格式暂不支持预览" : "选择课程文档" }}</strong>
      <span>{{ media ? "当前支持 PDF、DOCX、XLS/XLSX、PPTX" : message }}</span>
      <el-button v-if="!media" type="primary" @click="emit('addFolder')">添加文件夹</el-button>
    </div>
  </section>
</template>

<style scoped lang="scss">
.document-preview {
  display: grid;
  min-width: 0;
  min-height: 0;
  height: 100%;
  grid-template-rows: auto minmax(0, 1fr);
  overflow: hidden;
  border: 1px solid rgba(148, 163, 184, 0.18);
  border-radius: 6px;
  background: rgba(16, 23, 34, 0.72);
}

.document-header {
  display: flex;
  min-width: 0;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px;
  border-bottom: 1px solid rgba(148, 163, 184, 0.14);
}

.document-title {
  display: flex;
  min-width: 0;
  align-items: center;
  gap: 10px;
  color: #d8e5f7;
}

.document-title > .el-icon {
  flex: 0 0 auto;
  width: 30px;
  height: 30px;
  border: 1px solid rgba(59, 130, 246, 0.28);
  border-radius: 6px;
  background: rgba(59, 130, 246, 0.12);
  color: #9cc5ff;
}

.document-title div {
  min-width: 0;
}

.document-title strong,
.document-title span {
  overflow: hidden;
  display: block;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.document-title strong {
  color: #f8fafc;
  font-size: 14px;
  font-weight: 750;
}

.document-title span {
  margin-top: 4px;
  color: #93a6bd;
  font-size: 11px;
}

.document-action {
  display: inline-flex;
  flex: 0 0 auto;
  align-items: center;
  gap: 6px;
  min-height: 30px;
  padding: 0 10px;
  border: 1px solid rgba(59, 130, 246, 0.24);
  border-radius: 6px;
  background: rgba(59, 130, 246, 0.12);
  color: #cfe1ff;
  font-size: 12px;
  cursor: pointer;
}

.document-action:hover {
  border-color: rgba(59, 130, 246, 0.44);
  background: rgba(59, 130, 246, 0.2);
  color: #ffffff;
}

.document-body {
  min-width: 0;
  min-height: 0;
  height: 100%;
  overflow: hidden;
  background: #858b94;
}

.document-viewer {
  width: 100%;
  height: 100%;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}

.office-viewer {
  width: 100%;
  height: 100%;
  min-height: 0;
}

.document-viewer :deep(.vue-office-pdf),
.document-viewer :deep(.vue-office-docx),
.document-viewer :deep(.vue-office-excel),
.document-viewer :deep(.vue-office-pptx) {
  width: 100%;
  height: 100% !important;
  min-height: 0;
}

.document-viewer :deep(.vue-office-pdf) {
  overflow: auto !important;
  background: #858b94;
}

.document-viewer :deep(canvas) {
  display: block;
  max-width: 100%;
  margin: 0 auto;
}

.document-empty {
  display: grid;
  place-content: center;
  justify-items: center;
  gap: 10px;
  padding: 28px;
  color: #cbd5e1;
  text-align: center;
}

.document-empty strong {
  color: #f8fafc;
  font-size: 16px;
}

.document-empty span {
  max-width: 360px;
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
</style>
