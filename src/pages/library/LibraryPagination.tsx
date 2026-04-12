import React from 'react';
import { useLibraryStore } from '../../stores/libraryStore';

export function LibraryPagination() {
  const { query, totalCount, setPage, isLoading } = useLibraryStore();
  
  const totalPages = Math.ceil(totalCount / query.pageSize) || 1;
  const isFirst = query.page <= 1;
  const isLast = query.page >= totalPages;

  if (totalCount === 0) return null;

  return (
    <div style={{ 
      display: 'flex', 
      justifyContent: 'center', 
      alignItems: 'center', 
      gap: '1rem',
      padding: '1rem 0',
      marginTop: 'auto',
      borderTop: '1px solid #27272a'
    }}>
      <button 
        onClick={() => setPage(query.page - 1)}
        disabled={isFirst || isLoading}
        style={{
          background: isFirst ? 'transparent' : '#27272a',
          color: isFirst ? '#555' : '#f4f4f5',
          border: '1px solid #3f3f46',
          borderRadius: '6px',
          padding: '0.4rem 1rem',
          cursor: isFirst ? 'default' : 'pointer'
        }}
      >
        ← Previous
      </button>
      
      <span style={{ fontSize: '0.9rem', color: '#a1a1aa' }}>
        Page {query.page} of {totalPages}
      </span>
      
      <button 
        onClick={() => setPage(query.page + 1)}
        disabled={isLast || isLoading}
        style={{
          background: isLast ? 'transparent' : '#27272a',
          color: isLast ? '#555' : '#f4f4f5',
          border: '1px solid #3f3f46',
          borderRadius: '6px',
          padding: '0.4rem 1rem',
          cursor: isLast ? 'default' : 'pointer'
        }}
      >
        Next →
      </button>
    </div>
  );
}
