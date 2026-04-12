import React, { useEffect, useState, useRef } from 'react';
import { useLibraryStore } from '../../stores/libraryStore';

export function LibrarySearchBar() {
  const { query, setSearchTerm } = useLibraryStore();
  const [localTerm, setLocalTerm] = useState(query.searchTerm || '');
  const debounceRef = useRef<NodeJS.Timeout | null>(null);

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value;
    setLocalTerm(val);

    if (debounceRef.current) clearTimeout(debounceRef.current);
    
    debounceRef.current = setTimeout(() => {
      setSearchTerm(val);
    }, 300);
  };

  return (
    <div style={{ position: 'relative', width: '300px' }}>
      <input 
        type="text" 
        placeholder="Search files..." 
        value={localTerm}
        onChange={handleChange}
        style={{
          width: '100%',
          padding: '0.6rem 1rem 0.6rem 2.5rem',
          background: '#18181b',
          border: '1px solid #3f3f46',
          borderRadius: '20px',
          color: '#f4f4f5',
          fontSize: '0.9rem',
          outline: 'none',
          boxSizing: 'border-box'
        }}
        onFocus={e => e.currentTarget.style.borderColor = '#6366f1'}
        onBlur={e => e.currentTarget.style.borderColor = '#3f3f46'}
      />
      <span style={{ position: 'absolute', left: '12px', top: '50%', transform: 'translateY(-50%)', opacity: 0.5 }}>
        🔍
      </span>
    </div>
  );
}
