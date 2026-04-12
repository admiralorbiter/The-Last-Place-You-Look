import { create } from 'zustand';

interface AppStore {
  appReady: boolean;
  appVersion: string | null;
  dbStatus: string | null;
  setAppReady: (ready: boolean) => void;
  setAppVersion: (version: string) => void;
  setDbStatus: (status: string) => void;
}

export const useAppStore = create<AppStore>((set) => ({
  appReady: false,
  appVersion: null,
  dbStatus: null,
  setAppReady: (ready) => set({ appReady: ready }),
  setAppVersion: (version) => set({ appVersion: version }),
  setDbStatus: (status) => set({ dbStatus: status }),
}));
