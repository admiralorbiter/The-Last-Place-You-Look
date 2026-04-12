import React from 'react';
import { useLibraryStore } from '../../stores/libraryStore';

const formatBytes = (bytes: number) => {
  if (!bytes) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
};

export function LibraryStatsBar() {
  const { stats } = useLibraryStore();

  if (!stats) return null;

  return (
    <div style={{ 
      background: '#18181b', 
      border: '1px solid #27272a', 
      borderRadius: '8px', 
      padding: '0.8rem 1.5rem',
      display: 'flex',
      gap: '2rem',
      fontSize: '0.9rem',
      color: '#a1a1aa'
    }}>
      <div><strong style={{ color: '#e4e4e7' }}>{stats.totalFiles.toLocaleString()}</strong> Files Cataloged</div>
      <div><strong style={{ color: '#e4e4e7' }}>{formatBytes(stats.totalSizeBytes)}</strong> Total Size</div>
      <div><strong style={{ color: '#e4e4e7' }}>{stats.sourcesCount}</strong> Sources</div>
    </div>
  );
}
