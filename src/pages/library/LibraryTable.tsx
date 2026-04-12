import React from 'react';
import { useLibraryStore, SortBy, LibraryItem } from '../../stores/libraryStore';
import { useSourceStore } from '../../stores/sourceStore';
import { openPath } from '@tauri-apps/plugin-opener';
import { invoke } from '@tauri-apps/api/core';

const formatBytes = (bytes: number) => {
  if (!bytes) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
};

export function LibraryTable() {
  const { items, isLoading, query, setSort, selectedItemId, setSelectedItemId } = useLibraryStore();
  const { sources } = useSourceStore();

  const getAbsolutePath = (item: LibraryItem) => {
    const source = sources.find(s => s.id === item.sourceId);
    if (!source || !source.current_mount_path) return null;
    return source.current_mount_path.replace(/\\$/, '') + '\\' + item.volumeRelativePath;
  };

  // onDoubleClick removed in favor of single-click to view panel

  const handleContextMenu = async (e: React.MouseEvent, item: LibraryItem) => {
    e.preventDefault();
    const path = getAbsolutePath(item);
    if (path) {
      try { await invoke('reveal_in_explorer', { path }); } catch (e) { alert(`Failed to reveal in explorer: ${e}`); }
    }
  };

  if (isLoading && items.length === 0) {
    return <div style={{ color: '#888', textAlign: 'center', padding: '4rem' }}>Loading library...</div>;
  }

  if (items.length === 0) {
    return <div style={{ color: '#888', textAlign: 'center', padding: '4rem' }}>No files match your search criteria.</div>;
  }

  const getSortIcon = (field: SortBy) => {
    if (query.sortBy !== field) return <span style={{ opacity: 0 }}>⇅</span>;
    return query.sortDir === 'asc' ? '↑' : '↓';
  };

  const HeaderCell = ({ field, label, width, align = 'left' }: { field: SortBy, label: string, width?: string, align?: 'left' | 'right' }) => (
    <th 
      onClick={() => setSort(field)}
      style={{ 
        textAlign: align, 
        padding: '0.75rem 1rem', 
        cursor: 'pointer',
        userSelect: 'none',
        color: query.sortBy === field ? '#f4f4f5' : '#a1a1aa',
        borderBottom: '1px solid #3f3f46',
        background: '#09090b',
        position: 'sticky',
        top: 0,
        zIndex: 10,
        width: width,
        transition: 'color 0.2s'
      }}
      onMouseOver={e => e.currentTarget.style.color = '#f4f4f5'}
      onMouseOut={e => { if(query.sortBy !== field) e.currentTarget.style.color = '#a1a1aa'; }}
    >
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: align === 'right' ? 'flex-end' : 'flex-start', gap: '0.4rem', fontSize: '0.8rem', textTransform: 'uppercase', letterSpacing: '0.05em' }}>
        {label} 
        <span style={{ fontSize: '0.9rem', color: query.sortBy === field ? '#8b5cf6' : '#555' }}>
          {getSortIcon(field)}
        </span>
      </div>
    </th>
  );

  return (
    <div style={{ overflow: 'auto', flex: 1, border: '1px solid #27272a', borderRadius: '8px', background: '#18181b' }}>
      <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '0.85rem' }}>
        <thead>
          <tr>
            <HeaderCell field="fileName" label="Name" width="35%" />
            <HeaderCell field="extension" label="Type" width="10%" />
            <HeaderCell field="sizeBytes" label="Size" width="10%" align="right" />
            <HeaderCell field="modifiedAt" label="Date Modified" width="15%" />
            <th style={{ 
                textAlign: 'left', padding: '0.75rem 1rem', color: '#a1a1aa', borderBottom: '1px solid #3f3f46', 
                background: '#09090b', position: 'sticky', top: 0, zIndex: 10, fontSize: '0.8rem', textTransform: 'uppercase', letterSpacing: '0.05em'
            }}>Full Path</th>
          </tr>
        </thead>
        <tbody>
          {items.map((item, i) => (
            <tr 
              key={item.id} 
              style={{
                background: item.id === selectedItemId ? 'rgba(99, 102, 241, 0.2)' : (i % 2 === 0 ? 'transparent' : 'rgba(255,255,255,0.02)'),
                borderBottom: '1px solid #27272a',
                transition: 'background 0.2s',
                opacity: item.currentlyMounted ? 1 : 0.5,
                cursor: 'pointer',
                borderLeft: item.id === selectedItemId ? '3px solid #6366f1' : '3px solid transparent'
              }}
              onMouseOver={e => { if(item.id !== selectedItemId) e.currentTarget.style.background = 'rgba(99, 102, 241, 0.1)' }}
              onMouseOut={e => { if(item.id !== selectedItemId) e.currentTarget.style.background = i % 2 === 0 ? 'transparent' : 'rgba(255,255,255,0.02)' }}
              onClick={() => setSelectedItemId(item.id)}
              onContextMenu={(e) => handleContextMenu(e, item)}
            >
              <td style={{ padding: '0.6rem 1rem', color: '#e4e4e7', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis', maxWidth: '300px' }} title={item.fileName}>
                {item.fileName}
              </td>
              <td style={{ padding: '0.6rem 1rem', color: '#a1a1aa' }}>
                {item.extension ? item.extension.toUpperCase() : '--'}
              </td>
              <td style={{ padding: '0.6rem 1rem', color: '#a1a1aa', textAlign: 'right' }}>
                {formatBytes(item.sizeBytes)}
              </td>
              <td style={{ padding: '0.6rem 1rem', color: '#a1a1aa' }}>
                {new Date(item.modifiedAt).toLocaleString(undefined, { year: '2-digit', month: 'numeric', day: 'numeric', hour: 'numeric', minute: '2-digit' })}
              </td>
              <td style={{ padding: '0.6rem 1rem', color: '#71717a', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis', maxWidth: '300px', direction: 'rtl', textAlign: 'left' }} title={item.volumeRelativePath}>
                {'‎'}{item.volumeRelativePath}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
