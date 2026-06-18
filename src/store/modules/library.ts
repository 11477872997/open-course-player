import { defineStore } from "pinia";
import type { MediaLibraryRoot, MediaTreeNode, SelectedMedia } from "../../types/media";

interface LibraryState {
  roots: MediaLibraryRoot[];
  activeRootId: string;
  selectedMedia: SelectedMedia | null;
}

function rootNameFromPath(path: string) {
  const normalized = path.replace(/[\\\/]+$/, "");
  const parts = normalized.split(/[\\\/]/).filter(Boolean);
  return parts.at(-1) || path;
}

function countNodes(nodes: MediaTreeNode[]) {
  let totalFiles = 0;
  let playableFiles = 0;

  const walk = (items: MediaTreeNode[]) => {
    for (const item of items) {
      if (item.kind !== "folder") {
        totalFiles += 1;
        if (item.playable) playableFiles += 1;
      }
      if (item.children?.length) walk(item.children);
    }
  };

  walk(nodes);
  return { totalFiles, playableFiles };
}

export const useLibraryStore = defineStore("library", {
  state: (): LibraryState => ({
    roots: [],
    activeRootId: "",
    selectedMedia: null
  }),
  getters: {
    activeRoot(state): MediaLibraryRoot | null {
      return state.roots.find((root) => root.id === state.activeRootId) ?? state.roots[0] ?? null;
    },
    activeNodes(): MediaTreeNode[] {
      return this.activeRoot?.nodes ?? [];
    },
    totalPlayableFiles(state): number {
      return state.roots.reduce((sum, root) => sum + root.playableFiles, 0);
    }
  },
  actions: {
    upsertRoot(rootPath: string, nodes: MediaTreeNode[]) {
      const id = rootPath;
      const counts = countNodes(nodes);
      const root: MediaLibraryRoot = {
        id,
        name: rootNameFromPath(rootPath),
        path: rootPath,
        nodes,
        ...counts
      };

      const index = this.roots.findIndex((item) => item.id === id);
      if (index >= 0) {
        this.roots[index] = root;
      } else {
        this.roots.push(root);
      }

      this.activeRootId = id;
    },
    setActiveRoot(id: string) {
      this.activeRootId = id;
    },
    selectMedia(media: SelectedMedia) {
      this.selectedMedia = media;
    },
    clearSelectedMedia() {
      this.selectedMedia = null;
    },
    removeRoot(id: string) {
      this.roots = this.roots.filter((root) => root.id !== id);
      if (this.activeRootId === id) {
        this.activeRootId = this.roots[0]?.id ?? "";
      }
    }
  }
});
