import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export interface StorageSource {
  id: string;
  display_name: string;
  source_kind: string;
  stable_volume_identity: string;
  current_mount_path: string | null;
  currently_mounted: boolean;
  quarantine_root: string | null;
  created_at: string;
  files_indexed: number;  // 0 = never scanned, >0 = has catalog data
}

interface SourceStore {
  sources: StorageSource[];
  setSources: (sources: StorageSource[]) => void;
  initSources: () => Promise<void>;
}

export const useSourceStore = create<SourceStore>((set) => ({
  sources: [],
  setSources: (sources) => set({ sources }),
  initSources: async () => {
    try {
      const dbSources = await invoke<StorageSource[]>('list_storage_sources');
      set({ sources: dbSources });
    } catch (e) {
      console.error('Failed to list sources:', e);
    }

    listen('sources://status_updated', async () => {
      try {
        const dbSources = await invoke<StorageSource[]>('list_storage_sources');
        set({ sources: dbSources });
      } catch (e) {
        console.error('Failed to reload sources:', e);
      }
    });
  },
}));
