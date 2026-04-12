import React, { useEffect } from 'react';
import { useLibraryStore } from '../../stores/libraryStore';
import { useSourceStore } from '../../stores/sourceStore';

export function LibraryFilterSidebar() {
  const { query, setFilter, toggleSource, toggleExtension, extensionFacets } = useLibraryStore();
  const { sources, initSources } = useSourceStore();

  useEffect(() => {
    initSources();
  }, [initSources]);

  return (
    <div style={{ width: '240px', display: 'flex', flexDirection: 'column', gap: '1.5rem', flexShrink: 0, overflowY: 'auto', paddingRight: '0.5rem' }}>
      {/* Sources Filter */}
      <div>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.8rem' }}>
          <h4 style={{ margin: 0, color: '#e4e4e7', fontSize: '0.9rem', textTransform: 'uppercase', letterSpacing: '1px' }}>Sources</h4>
          {query.sourceIds?.length > 0 && (
            <button onClick={() => setFilter('sourceIds', [])} style={{ background: 'transparent', border: 'none', color: '#8b5cf6', fontSize: '0.75rem', cursor: 'pointer', padding: 0 }}>Clear</button>
          )}
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: '0.4rem' }}>
          {sources.map(s => (
            <label key={s.id} style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', color: '#a1a1aa', fontSize: '0.9rem', cursor: 'pointer' }}>
              <input 
                type="checkbox" 
                checked={query.sourceIds?.includes(s.id) || false} 
                onChange={() => toggleSource(s.id)} 
              /> 
               <span style={{ 
                 whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis', maxWidth: '140px' 
               }} title={s.display_name}>{s.display_name}</span> 
               {!s.currently_mounted && <span style={{ fontSize: '0.7rem', color: '#ef4444' }}>(Offline)</span>}
            </label>
          ))}
        </div>
      </div>

      {/* Extension Group Filter */}
      <div>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.8rem' }}>
          <h4 style={{ margin: 0, color: '#e4e4e7', fontSize: '0.9rem', textTransform: 'uppercase', letterSpacing: '1px' }}>Extensions</h4>
          {query.extensions?.length > 0 && (
            <button onClick={() => setFilter('extensions', [])} style={{ background: 'transparent', border: 'none', color: '#8b5cf6', fontSize: '0.75rem', cursor: 'pointer', padding: 0 }}>Clear</button>
          )}
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: '0.4rem' }}>
          {extensionFacets.map(facet => (
            <label key={facet.extension} style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', color: '#a1a1aa', fontSize: '0.9rem', cursor: 'pointer' }}>
              <input 
                type="checkbox" 
                checked={query.extensions?.includes(facet.extension) || false} 
                onChange={() => toggleExtension(facet.extension)} 
              /> 
              <span style={{ textTransform: 'uppercase' }}>{facet.extension}</span>
              <span style={{ fontSize: '0.7rem', opacity: 0.5, marginLeft: 'auto' }}>({facet.count.toLocaleString()})</span>
            </label>
          ))}
          {extensionFacets.length === 0 && (
            <span style={{ fontSize: '0.8rem', color: '#555', fontStyle: 'italic' }}>No extensions found</span>
          )}
        </div>
      </div>
      
    </div>
  );
}
