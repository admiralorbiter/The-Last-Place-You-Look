import React, { useEffect, useState } from 'react';
import { useSourceStore } from '../stores/sourceStore';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

export function Sources() {
  const { sources, initSources } = useSourceStore();
  const [addingName, setAddingName] = useState('');

  useEffect(() => {
    initSources();
  }, [initSources]);

  const handleAddSource = async () => {
    try {
      const selectedPath = await open({
        directory: true,
        multiple: false,
        title: 'Select a drive or folder to add as a source'
      });

      if (selectedPath) {
        const name = prompt('Enter a display name for this source:', 'My Drive');
        if (name) {
          await invoke('add_storage_source', {
            path: Array.isArray(selectedPath) ? selectedPath[0] : selectedPath,
            displayName: name,
            sourceKind: 'removable', // Default to removable in UI for MVP
          });
          // State auto-refreshes when we call initSources or event updates? We should probably manually refresh here.
          initSources();
        }
      }
    } catch (e) {
      alert(`Failed to add source: ${e}`);
    }
  };

  const handleRemoveSource = async (id: string, name: string) => {
    if (confirm(`Remove ${name}? This will keep all scanned catalog data.`)) {
      try {
        await invoke('remove_storage_source', { sourceId: id });
        initSources();
      } catch (e) {
        alert(`Failed to remove source: ${e}`);
      }
    }
  };

  return (
    <div style={{ marginTop: '2rem' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <h2>Storage Sources</h2>
        <button onClick={handleAddSource} style={{ padding: '0.5rem 1rem' }}>
          + Add Source
        </button>
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem', marginTop: '1rem' }}>
        {sources.length === 0 && <p>No sources registered yet.</p>}
        {sources.map(source => (
          <div key={source.id} style={{ border: '1px solid #555', padding: '1rem', borderRadius: '4px', backgroundColor: '#1a1a1a' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between' }}>
              <h3>{source.display_name}</h3>
              <button 
                onClick={() => handleRemoveSource(source.id, source.display_name)}
                style={{ backgroundColor: 'transparent', border: '1px solid #c0392b', color: '#e74c3c' }}
              >
                Remove
              </button>
            </div>
            
            <p>
              <span style={{ 
                display: 'inline-block',
                padding: '0.2rem 0.5rem', 
                borderRadius: '4px',
                fontSize: '0.8rem',
                backgroundColor: source.currently_mounted ? '#27ae60' : '#f39c12',
                color: '#fff',
                marginRight: '0.5rem'
              }}>
                {source.currently_mounted ? 'Online' : 'Offline'}
              </span>
              <span style={{ fontSize: '0.8rem', color: '#aaa', backgroundColor: '#333', padding: '0.2rem 0.5rem', borderRadius: '4px' }}>
                {source.source_kind}
              </span>
            </p>

            <div style={{ fontSize: '0.9rem', color: '#ccc', marginTop: '0.5rem' }}>
              <p><strong>Mount Path:</strong> {source.current_mount_path || 'N/A'}</p>
              <p><strong>Quarantine Root:</strong> {source.quarantine_root || 'N/A'}</p>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
