import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export interface ScanProgress {
  source_id: string;
  status: 'running' | 'completed' | 'failed' | 'cancelled';
  stage: number;
  files_found: number;
  files_inserted: number;
  bytes_found: number;
  total_used_bytes: number;
}

interface PipelineStore {
  activeScans: Record<string, ScanProgress>;
  
  // Actions
  fetchInitialState: () => Promise<void>;
  startScan: (sourceId: string) => Promise<void>;
  cancelScan: (sourceId: string) => Promise<void>;
}

// Guard prevents re-registering the listener if the store re-initializes
let pipelineListenerSetup = false;

export const usePipelineStore = create<PipelineStore>((set) => ({
  activeScans: {},

  fetchInitialState: async () => {
    // Subscribe to live scan progress events lazily — MUST NOT be at module-init
    // time because the Tauri IPC bridge may not yet exist, which would throw a
    // synchronous exception that crashes the whole JS module graph.
    if (!pipelineListenerSetup) {
      pipelineListenerSetup = true;
      try {
        await listen<ScanProgress>("pipeline://progress", (event) => {
          set((state) => ({
            activeScans: {
              ...state.activeScans,
              [event.payload.source_id]: event.payload
            }
          }));
        });
      } catch (e) {
        console.error("Failed to set up pipeline progress listener", e);
      }
    }

    try {
      const statuses = await invoke<ScanProgress[]>("get_scan_status");
      const map: Record<string, ScanProgress> = {};
      for (const status of statuses) {
        map[status.source_id] = status;
      }
      set({ activeScans: map });
    } catch (e) {
      console.error("Failed to fetch scan status", e);
    }
  },

  startScan: async (sourceId: string) => {
    try {
      // Optimistically set to running so UI reacts immediately
      set((state) => ({
        activeScans: {
          ...state.activeScans,
          [sourceId]: {
            source_id: sourceId,
            status: 'running',
            stage: 1,
            files_found: 0,
            files_inserted: 0,
            bytes_found: 0,
            total_used_bytes: 0
          }
        }
      }));

      await invoke("start_scan", { sourceId });
    } catch (e) {
      console.error("Failed to start scan", e);
      // Remove optimistic state on failure
      set((state) => {
        const next = { ...state.activeScans };
        delete next[sourceId];
        return { activeScans: next };
      });
    }
  },

  cancelScan: async (sourceId: string) => {
    try {
      await invoke("cancel_scan", { sourceId });
    } catch (e) {
      console.error("Failed to cancel scan", e);
    }
  }
}));
