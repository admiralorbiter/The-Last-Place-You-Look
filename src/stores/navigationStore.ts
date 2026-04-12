import { create } from 'zustand';

export type Page = 'home' | 'library' | 'sources' | 'duplicates' | 'settings';

interface NavigationStore {
  activePage: Page;
  setActivePage: (page: Page) => void;
}

export const useNavigationStore = create<NavigationStore>((set) => ({
  activePage: 'sources', // temporarily default to sources so they start at a familiar place
  setActivePage: (page) => set({ activePage: page }),
}));
