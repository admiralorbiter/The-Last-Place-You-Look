import React, { useEffect } from 'react';
import { useLibraryStore } from '../stores/libraryStore';
import { LibrarySearchBar } from './library/LibrarySearchBar';
import { LibraryFilterSidebar } from './library/LibraryFilterSidebar';
import { LibraryTable } from './library/LibraryTable';
import { LibraryPagination } from './library/LibraryPagination';
import { LibraryStatsBar } from './library/LibraryStatsBar';

export function Library() {
  const { fetchStats, fetchPage } = useLibraryStore();

  useEffect(() => {
    fetchStats();
    fetchPage();
  }, [fetchStats, fetchPage]);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', fontFamily: "'Inter', sans-serif" }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1.5rem' }}>
        <h2 style={{ margin: 0, fontWeight: 700, fontSize: '1.8rem', color: '#e0e0e0' }}>Unified Library</h2>
        <LibrarySearchBar />
      </div>

      <LibraryStatsBar />

      <div style={{ display: 'flex', gap: '2rem', flex: 1, overflow: 'hidden', marginTop: '1.5rem' }}>
        <LibraryFilterSidebar />
        
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
          <LibraryTable />
          <LibraryPagination />
        </div>
      </div>
    </div>
  );
}
