import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

export interface LibraryItem {
  id: string;
  sourceId: string;
  sourceName: string;
  currentlyMounted: boolean;
  fileName: string;
  volumeRelativePath: string;
  extension: string | null;
  sizeBytes: number;
  modifiedAt: string;
  deletedAt: string | null;
}

export interface FileDetail {
  id: string;
  assetId: string | null;
  sourceId: string;
  sourceName: string;
  currentlyMounted: boolean;
  fileName: string;
  currentPath: string | null;
  volumeRelativePath: string;
  extension: string | null;
  sizeBytes: number;
  modifiedAt: string;
  createdAtFs: string | null;
  stage2At: string | null;
  blake3Hash: string | null;
  quarantineStatus: string;
}

export type SortBy = 'modifiedAt' | 'sizeBytes' | 'fileName' | 'extension';
export type SortDir = 'asc' | 'desc';

export interface LibraryQuery {
  searchTerm: string | null;
  sourceIds: string[];
  extensions: string[];
  statusFilter: string | null; // "online" or "all"
  sortBy: SortBy;
  sortDir: SortDir;
  page: number;
  pageSize: number;
}

export interface LibraryPage {
  items: LibraryItem[];
  totalCount: number;
  page: number;
  pageSize: number;
  extensionFacets: [string, number][];
}

export interface LibraryStats {
  totalFiles: number;
  totalSizeBytes: number;
  sourcesCount: number;
}

interface LibraryStore {
  items: LibraryItem[];
  totalCount: number;
  isLoading: boolean;
  stats: LibraryStats | null;
  query: LibraryQuery;
  extensionFacets: Array<{extension: string, count: number}>;
  selectedItemId: string | null;
  
  // Internal request tracking for debounce staleness
  currentRequestId: number;
  
  // Actions
  fetchStats: () => Promise<void>;
  fetchPage: () => Promise<void>;
  setSearchTerm: (term: string) => void;
  toggleSource: (sourceId: string) => void;
  toggleExtension: (extension: string) => void;
  setFilter: (key: keyof LibraryQuery, value: any) => void;
  setSort: (sortBy: SortBy) => void;
  setPage: (page: number) => void;
  setSelectedItemId: (id: string | null) => void;
}

const defaultQuery: LibraryQuery = {
  searchTerm: null,
  sourceIds: [],
  extensions: [],
  statusFilter: null,
  sortBy: 'modifiedAt',
  sortDir: 'desc',
  page: 1,
  pageSize: 100,
};

export const useLibraryStore = create<LibraryStore>((set, get) => ({
  items: [],
  totalCount: 0,
  isLoading: false,
  stats: null,
  query: defaultQuery,
  extensionFacets: [],
  selectedItemId: null,
  currentRequestId: 0,

  fetchStats: async () => {
    try {
      const stats = await invoke<LibraryStats>('get_library_stats');
      set({ stats });
    } catch (e) {
      console.error("Failed to fetch library stats", e);
    }
  },

  fetchPage: async () => {
    const { query, currentRequestId } = get();
    const requestId = currentRequestId + 1;
    set({ isLoading: true, currentRequestId: requestId });

    try {
      // Use search_library if there's a search term, else list_library
      const command = (query.searchTerm && query.searchTerm.trim() !== '') ? 'search_library' : 'list_library';
      
      const page = await invoke<LibraryPage>(command, { query });
      
      // Prevent stale overwrite
      if (get().currentRequestId === requestId) {
        set({
          items: page.items,
          totalCount: page.totalCount,
          extensionFacets: page.extensionFacets.map(f => ({ extension: f[0], count: f[1] })),
          isLoading: false,
        });
      }
    } catch (e) {
      console.error(`Failed to execute ${get().query.searchTerm ? 'search' : 'list'}`, e);
      if (get().currentRequestId === requestId) {
        set({ items: [], totalCount: 0, extensionFacets: [], isLoading: false });
      }
    }
  },

  setSearchTerm: (term: string) => {
    const newQuery = { ...get().query, searchTerm: term, page: 1 };
    set({ query: newQuery });
    get().fetchPage();
  },

  toggleSource: (sourceId: string) => {
    const current = get().query.sourceIds;
    const newSources = current.includes(sourceId) 
      ? current.filter(id => id !== sourceId)
      : [...current, sourceId];
    set({ query: { ...get().query, sourceIds: newSources, page: 1 } });
    get().fetchPage();
  },

  toggleExtension: (extension: string) => {
    const current = get().query.extensions;
    const newExts = current.includes(extension) 
      ? current.filter(e => e !== extension)
      : [...current, extension];
    set({ query: { ...get().query, extensions: newExts, page: 1 } });
    get().fetchPage();
  },

  setFilter: (key, value) => {
    const newQuery = { ...get().query, [key]: value, page: 1 };
    set({ query: newQuery });
    get().fetchPage();
  },

  setSort: (sortBy: SortBy) => {
    const current = get().query;
    let sortDir: SortDir = 'desc';
    
    // Toggle direction if clicking same column
    if (current.sortBy === sortBy) {
      sortDir = current.sortDir === 'asc' ? 'desc' : 'asc';
    } else {
      // Defaults: size/modified default to desc. name/ext default to asc.
      sortDir = (sortBy === 'modifiedAt' || sortBy === 'sizeBytes') ? 'desc' : 'asc';
    }
    
    set({ query: { ...current, sortBy, sortDir, page: 1 } });
    get().fetchPage();
  },

  setPage: (page: number) => {
    set({ query: { ...get().query, page } });
    get().fetchPage();
  },

  setSelectedItemId: (id: string | null) => {
    set({ selectedItemId: id });
  }
}));
