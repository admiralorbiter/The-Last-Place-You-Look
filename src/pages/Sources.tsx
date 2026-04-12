import React, { useEffect } from 'react';
import { useSourceStore } from '../stores/sourceStore';
import { usePipelineStore } from '../stores/pipelineStore';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

const formatBytes = (bytes: number) => {
  if (!bytes) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
};

export function Sources() {
  const { sources, initSources } = useSourceStore();
  const { activeScans, startScan, cancelScan, fetchInitialState } = usePipelineStore();

  const handleStartHashing = async (sourceId: string) => {
    try {
      await invoke('start_hashing', { sourceId });
    } catch (e) {
      alert(`Failed to start hashing: ${e}`);
    }
  };

  useEffect(() => {
    initSources();
    fetchInitialState();
  }, [initSources, fetchInitialState]);

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
            sourceKind: 'removable',
          });
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
    <div style={{ padding: '2rem', maxWidth: '900px', margin: '0 auto', fontFamily: "'Inter', sans-serif" }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '2rem' }}>
        <h2 style={{ margin: 0, fontWeight: 700, fontSize: '1.8rem', color: '#e0e0e0' }}>Storage Sources</h2>
        <button 
          onClick={handleAddSource} 
          style={{ 
            padding: '0.6rem 1.2rem', 
            background: 'linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%)',
            color: 'white', 
            border: 'none', 
            borderRadius: '8px',
            fontWeight: 600,
            cursor: 'pointer',
            boxShadow: '0 4px 15px rgba(99, 102, 241, 0.3)',
            transition: 'all 0.2s ease'
          }}
          onMouseOver={e => e.currentTarget.style.transform = 'translateY(-2px)'}
          onMouseOut={e => e.currentTarget.style.transform = 'translateY(0)'}
        >
          + Add Source
        </button>
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
        {sources.length === 0 && (
          <div style={{ textAlign: 'center', padding: '4rem', color: '#888', background: '#18181b', borderRadius: '12px', border: '1px dashed #333' }}>
            <p style={{ fontSize: '1.1rem', marginBottom: '1rem' }}>No storage sources connected</p>
            <p style={{ fontSize: '0.9rem', color: '#555' }}>Click the button above to register your first local or external drive.</p>
          </div>
        )}
        
        {sources.map(source => {
          const scan = activeScans[source.id];
          const isScanning = scan?.status === 'running';
          let percent = 0;
          if (scan) {
            if (scan.stage === 1 && scan.total_used_bytes) {
              percent = Math.min(100, Math.round((scan.bytes_found / scan.total_used_bytes) * 100));
            } else if (scan.stage === 2 && scan.files_found > 0) {
              percent = Math.min(100, Math.round((scan.files_inserted / scan.files_found) * 100));
            }
          }

          return (
            <div 
              key={source.id} 
              style={{ 
                background: '#18181b', 
                border: '1px solid #27272a', 
                borderRadius: '12px', 
                overflow: 'hidden',
                boxShadow: '0 4px 20px rgba(0,0,0,0.2)',
                transition: 'border-color 0.3s ease',
              }}
            >
              <div style={{ padding: '1.5rem', borderBottom: '1px solid #27272a' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1rem' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
                    <h3 style={{ margin: 0, fontSize: '1.3rem', color: '#f4f4f5' }}>{source.display_name}</h3>
                    <div style={{ display: 'flex', gap: '0.5rem' }}>
                      <span style={{ 
                        padding: '0.2rem 0.6rem', 
                        borderRadius: '20px',
                        fontSize: '0.75rem',
                        fontWeight: 600,
                        backgroundColor: source.currently_mounted ? 'rgba(16, 185, 129, 0.1)' : 'rgba(245, 158, 11, 0.1)',
                        color: source.currently_mounted ? '#10b981' : '#f59e0b',
                        border: `1px solid ${source.currently_mounted ? 'rgba(16, 185, 129, 0.2)' : 'rgba(245, 158, 11, 0.2)'}`
                      }}>
                        {source.currently_mounted ? 'Online' : 'Offline'}
                      </span>
                      <span style={{ 
                        fontSize: '0.75rem', color: '#a1a1aa', backgroundColor: '#27272a', 
                        padding: '0.2rem 0.6rem', borderRadius: '20px', border: '1px solid #3f3f46'
                      }}>
                        {source.source_kind === 'removable' ? 'USB / External' : 'Internal Disk'}
                      </span>
                    </div>
                  </div>
                  <button 
                    onClick={() => handleRemoveSource(source.id, source.display_name)}
                    style={{ 
                      backgroundColor: 'transparent', 
                      border: '1px solid #ef4444', 
                      color: '#ef4444',
                      padding: '0.4rem 0.8rem',
                      borderRadius: '6px',
                      cursor: 'pointer',
                      fontSize: '0.85rem',
                      transition: 'all 0.2s'
                    }}
                    onMouseOver={e => { e.currentTarget.style.backgroundColor = '#ef4444'; e.currentTarget.style.color = 'white'; }}
                    onMouseOut={e => { e.currentTarget.style.backgroundColor = 'transparent'; e.currentTarget.style.color = '#ef4444'; }}
                  >
                    Disconnect
                  </button>
                </div>
                
                <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1rem', fontSize: '0.9rem', color: '#a1a1aa' }}>
                  <div style={{ background: '#09090b', padding: '0.75rem', borderRadius: '8px' }}>
                    <span style={{ display: 'block', fontSize: '0.75rem', color: '#71717a', textTransform: 'uppercase', letterSpacing: '1px', marginBottom: '0.25rem' }}>Mount Root</span>
                    <span style={{ fontFamily: 'monospace', color: '#e4e4e7' }}>{source.current_mount_path || 'Disconnected'}</span>
                  </div>
                  <div style={{ background: '#09090b', padding: '0.75rem', borderRadius: '8px' }}>
                    <span style={{ display: 'block', fontSize: '0.75rem', color: '#71717a', textTransform: 'uppercase', letterSpacing: '1px', marginBottom: '0.25rem' }}>Quarantine Zone</span>
                    <span style={{ fontFamily: 'monospace', color: '#e4e4e7' }}>{source.quarantine_root || 'N/A'}</span>
                  </div>
                </div>
              </div>

              {/* Action / Scan Area */}
              {source.currently_mounted && (
                <div style={{ padding: '1.5rem', background: isScanning ? 'rgba(99, 102, 241, 0.05)' : 'transparent', transition: 'background 0.5s ease' }}>
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                    <div>
                      {!isScanning && (
                        <p style={{ margin: 0, color: source.files_indexed > 0 ? '#10b981' : '#a1a1aa', fontSize: '0.95rem' }}>
                          {source.files_indexed > 0
                            ? `Cataloged ${source.files_indexed.toLocaleString()} files`
                            : 'Ready to scan catalog inventory.'}
                        </p>
                      )}
                      {isScanning && scan?.stage === 1 && (
                        <div>
                          <p style={{ margin: 0, color: '#e0e0e0', fontWeight: 600 }}>Scanning Drive Inventory...</p>
                          <p style={{ margin: '0.2rem 0 0 0', color: '#888', fontSize: '0.85rem' }}>
                            {scan.files_found.toLocaleString()} files verified ({formatBytes(scan.bytes_found)} scanned)
                          </p>
                        </div>
                      )}
                      {isScanning && scan?.stage === 2 && (
                        <div>
                          <p style={{ margin: 0, color: '#e0e0e0', fontWeight: 600 }}>Background Hashing (Stage 2)...</p>
                          <p style={{ margin: '0.2rem 0 0 0', color: '#888', fontSize: '0.85rem' }}>
                            {scan.files_inserted.toLocaleString()} / {scan.files_found.toLocaleString()} files hashed
                          </p>
                        </div>
                      )}
                    </div>
                    
                    <div>
                      {!isScanning ? (
                        <>
                          <button 
                            onClick={() => startScan(source.id)} 
                            style={{ 
                              padding: '0.6rem 1.5rem', 
                              backgroundColor: '#27272a', 
                              color: 'white', 
                              border: '1px solid #3f3f46', 
                              borderRadius: '6px',
                              fontWeight: 600,
                              cursor: 'pointer',
                              transition: 'all 0.2s',
                              width: '100%'
                            }}
                            onMouseOver={e => e.currentTarget.style.backgroundColor = '#3f3f46'}
                            onMouseOut={e => e.currentTarget.style.backgroundColor = '#27272a'}
                          >
                            {source.files_indexed > 0 ? 'Rescan Index' : 'Start Full Scan'}
                          </button>
                          {!isScanning && (
                            <button
                              onClick={() => handleStartHashing(source.id)}
                              style={{
                                marginTop: '0.5rem',
                                width: '100%',
                                padding: '0.5rem',
                                backgroundColor: 'transparent',
                                color: '#8b5cf6',
                                border: '1px solid rgba(139,92,246,0.4)',
                                borderRadius: '6px',
                                fontWeight: 500,
                                cursor: 'pointer',
                                fontSize: '0.85rem',
                                transition: 'all 0.2s'
                              }}
                              onMouseOver={e => { e.currentTarget.style.backgroundColor = 'rgba(139,92,246,0.1)'; }}
                              onMouseOut={e => { e.currentTarget.style.backgroundColor = 'transparent'; }}
                            >
                              ⚡ Start Hashing Only
                            </button>
                          )}
                        </>
                      ) : (
                        <button 
                          onClick={() => cancelScan(source.id)}
                          style={{ 
                            padding: '0.4rem 1rem', 
                            backgroundColor: 'transparent', 
                            color: '#71717a', 
                            border: '1px solid #3f3f46', 
                            borderRadius: '6px',
                            cursor: 'pointer'
                          }}
                        >
                          Cancel Scan
                        </button>
                      )}
                    </div>
                  </div>

                  {/* Progress Bar */}
                  {isScanning && (
                    <div style={{ marginTop: '1.25rem' }}>
                      <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.8rem', color: '#a1a1aa', marginBottom: '0.4rem' }}>
                        <span>{scan?.stage === 1 ? 'Stage 1: Metadata Walk' : 'Stage 2: BLAKE3 Fingerprinting'}</span>
                        <span>
                          {scan?.stage === 1 
                            ? `${percent}% (${formatBytes(scan?.total_used_bytes || 0)} total)`
                            : `${percent}%`}
                        </span>
                      </div>
                      <div style={{ height: '6px', background: '#27272a', borderRadius: '3px', overflow: 'hidden' }}>
                        <div style={{ 
                          height: '100%', 
                          width: `${percent}%`, 
                          background: 'linear-gradient(90deg, #6366f1 0%, #8b5cf6 100%)',
                          borderRadius: '3px',
                          transition: 'width 0.3s cubic-bezier(0.4, 0, 0.2, 1)',
                          boxShadow: '0 0 10px rgba(139, 92, 246, 0.5)'
                        }} />
                      </div>
                    </div>
                  )}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
