import { defineStore } from "pinia";
import type { MediaTreeNode, SelectedMedia } from "../../types/media";

interface LibraryState {
  rootPath: string;
  nodes: MediaTreeNode[];
  selectedMedia: SelectedMedia | null;
}

export const useLibraryStore = defineStore("library", {
  state: (): LibraryState => ({
    rootPath: "",
    nodes: [],
    selectedMedia: null
  }),
  actions: {
    setRoot(rootPath: string, nodes: MediaTreeNode[]) {
      this.rootPath = rootPath;
      this.nodes = nodes;
    },
    selectMedia(media: SelectedMedia) {
      this.selectedMedia = media;
    }
  }
});
