import React, { useEffect, useState, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { openPath } from '@tauri-apps/plugin-opener';
import { useLibraryStore, FileDetail } from '../../stores/libraryStore';

const formatBytes = (bytes: number) => {
  if (!bytes) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
};

interface DuplicateEntry {
  id: string;
  file_name: string;
  current_path: string | null;
  source_name: string;
  size_bytes: number;
  confidence: 'confirmed' | 'probable';
}

interface DuplicateResult {
  confirmed: DuplicateEntry[];
  probable: DuplicateEntry[];
}

export function LibraryDetailPanel() {
  const { selectedItemId, setSelectedItemId } = useLibraryStore();
  const [fileDetail, setFileDetail] = useState<FileDetail | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [thumbnailSrc, setThumbnailSrc] = useState<string | null>(null);
  const [thumbError, setThumbError] = useState<boolean>(false);

  // Hash state
  const [isHashing, setIsHashing] = useState(false);
  const [localHash, setLocalHash] = useState<string | null>(null);
  const [hashError, setHashError] = useState<string | null>(null);

  // Duplicate state — loaded immediately (name+size), updated after hash
  const [duplicates, setDuplicates] = useState<DuplicateResult | null>(null);
  const [isDupeLoading, setIsDupeLoading] = useState(false);
  const [copied, setCopied] = useState(false);

  // Track which id we've started hashing so we don't double-trigger
  const hashingForId = useRef<string | null>(null);

  useEffect(() => {
    if (!selectedItemId) {
      setFileDetail(null);
      setThumbnailSrc(null);
      setThumbError(false);
      setLocalHash(null);
      setHashError(null);
      setDuplicates(null);
      setIsHashing(false);
      hashingForId.current = null;
      return;
    }

    let isMounted = true;
    setIsLoading(true);
    setError(null);
    setThumbnailSrc(null);
    setThumbError(false);
    setLocalHash(null);
    setHashError(null);
    setDuplicates(null);
    setIsHashing(false);
    hashingForId.current = null;

    invoke<FileDetail>('get_file_detail', { id: selectedItemId })
      .then(detail => {
        if (!isMounted) return;
        setFileDetail(detail);

        // Thumbnail
        if (detail.currentlyMounted) {
          invoke<string>('get_thumbnail', { id: selectedItemId })
            .then(uri => { if (isMounted) setThumbnailSrc(uri); })
            .catch(() => { if (isMounted) setThumbError(true); });
        }

        // Immediately load probable duplicates (name+size, no hash needed)
        setIsDupeLoading(true);
        invoke<DuplicateResult>('find_duplicates', { id: selectedItemId })
          .then(result => { if (isMounted) setDuplicates(result); })
          .finally(() => { if (isMounted) setIsDupeLoading(false); });

        // Auto-hash if not yet hashed and drive is online
        if (!detail.blake3Hash && detail.currentlyMounted && hashingForId.current !== selectedItemId) {
          hashingForId.current = selectedItemId;
          setIsHashing(true);
          invoke<string>('hash_single_file', { id: selectedItemId })
            .then(hash => {
              if (!isMounted) return;
              setLocalHash(hash);
              // Re-run duplicate search now that we have a confirmed hash
              invoke<DuplicateResult>('find_duplicates', { id: selectedItemId })
                .then(result => { if (isMounted) setDuplicates(result); });
            })
            .catch(e => { if (isMounted) setHashError(String(e)); })
            .finally(() => { if (isMounted) setIsHashing(false); });
        }
      })
      .catch(err => { if (isMounted) setError(String(err)); })
      .finally(() => { if (isMounted) setIsLoading(false); });

    return () => { isMounted = false; };
  }, [selectedItemId]);

  const handleCopyHash = (hash: string) => {
    navigator.clipboard.writeText(hash);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const displayHash = localHash ?? fileDetail?.blake3Hash;
  const totalDupes = (duplicates?.confirmed.length ?? 0) + (duplicates?.probable.length ?? 0);

  return (
    <div
      style={{
        position: 'absolute', top: 0, right: 0, bottom: 0, width: '400px',
        backgroundColor: '#18181b', borderLeft: '1px solid #27272a',
        transform: selectedItemId ? 'translateX(0)' : 'translateX(100%)',
        transition: 'transform 0.3s cubic-bezier(0.16, 1, 0.3, 1)',
        display: 'flex', flexDirection: 'column', zIndex: 10,
        boxShadow: selectedItemId ? '-5px 0 25px rgba(0,0,0,0.5)' : 'none',
      }}
    >
      {/* Header */}
      <div style={{ padding: '1rem 1.5rem', borderBottom: '1px solid #27272a', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <h3 style={{ margin: 0, color: '#f4f4f5', fontSize: '1.1rem', fontWeight: 600 }}>File Details</h3>
        <button
          onClick={() => setSelectedItemId(null)}
          style={{ background: 'transparent', border: 'none', color: '#a1a1aa', cursor: 'pointer', fontSize: '1.2rem', padding: '0.2rem 0.5rem', borderRadius: '4px' }}
          onMouseOver={e => e.currentTarget.style.backgroundColor = '#27272a'}
          onMouseOut={e => e.currentTarget.style.backgroundColor = 'transparent'}
        >
          &times;
        </button>
      </div>

      <div style={{ flex: 1, overflowY: 'auto', padding: '1.5rem' }}>
        {isLoading && (
          <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100px', color: '#a1a1aa' }}>
            Loading details...
          </div>
        )}
        {error && (
          <div style={{ color: '#f87171', padding: '1rem', backgroundColor: 'rgba(248,113,113,0.1)', borderRadius: '8px' }}>
            Failed to load: {error}
          </div>
        )}

        {!isLoading && !error && fileDetail && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '1.25rem' }}>

            {/* Preview */}
            <div style={{ height: '200px', backgroundColor: '#09090b', borderRadius: '8px', display: 'flex', justifyContent: 'center', alignItems: 'center', border: '1px solid #27272a', overflow: 'hidden' }}>
              {thumbnailSrc
                ? <img src={thumbnailSrc} alt="Preview" style={{ maxWidth: '100%', maxHeight: '100%', objectFit: 'contain' }} />
                : <span style={{ color: '#71717a', fontSize: '1.1rem', fontWeight: 600 }}>{thumbError ? 'NO PREVIEW' : (fileDetail.extension?.toUpperCase() || 'FILE')}</span>
              }
            </div>

            {/* Name */}
            <div>
              <h4 style={{ margin: '0 0 0.3rem 0', color: '#e4e4e7', fontSize: '1.05rem', wordBreak: 'break-word', lineHeight: 1.4 }}>
                {fileDetail.fileName}
              </h4>
              <p style={{ margin: 0, color: '#a1a1aa', fontSize: '0.88rem' }}>
                {formatBytes(fileDetail.sizeBytes)} &bull; {fileDetail.extension?.toUpperCase() || 'Unknown'}
              </p>
            </div>

            {/* Offline warning */}
            {!fileDetail.currentlyMounted && (
              <div style={{ backgroundColor: 'rgba(245,158,11,0.1)', border: '1px solid rgba(245,158,11,0.2)', padding: '0.6rem 0.75rem', borderRadius: '6px', color: '#fcd34d', fontSize: '0.82rem' }}>
                Drive '{fileDetail.sourceName}' is offline
              </div>
            )}

            <hr style={{ border: 0, borderTop: '1px solid #27272a', margin: 0 }} />

            {/* Metadata */}
            <div style={{ display: 'flex', flexDirection: 'column', gap: '0.8rem', fontSize: '0.84rem' }}>
              <Row label="Modified" value={new Date(fileDetail.modifiedAt).toLocaleString()} />
              <Row label="Created" value={fileDetail.createdAtFs ? new Date(fileDetail.createdAtFs).toLocaleString() : 'Unknown'} />
              <Row label="Source" value={fileDetail.sourceName} />
              <div style={{ display: 'flex', flexDirection: 'column', gap: '0.25rem' }}>
                <span style={{ color: '#71717a' }}>Location</span>
                <span style={{ color: '#e4e4e7', fontFamily: 'monospace', fontSize: '0.76rem', backgroundColor: '#09090b', padding: '0.35rem 0.5rem', borderRadius: '4px', wordBreak: 'break-all' }}>
                  {fileDetail.volumeRelativePath}
                </span>
              </div>

              {/* BLAKE3 */}
              <div style={{ display: 'flex', flexDirection: 'column', gap: '0.35rem' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                  <span style={{ color: '#71717a' }}>BLAKE3 Hash</span>
                  {isHashing && (
                    <span style={{ display: 'flex', alignItems: 'center', gap: '0.3rem', color: '#8b5cf6', fontSize: '0.78rem' }}>
                      <span style={{ display: 'inline-block', width: '10px', height: '10px', border: '2px solid rgba(139,92,246,0.3)', borderTopColor: '#8b5cf6', borderRadius: '50%', animation: 'spin 0.7s linear infinite' }} />
                      Hashing...
                    </span>
                  )}
                </div>

                {displayHash ? (
                  <div style={{ display: 'flex', gap: '0.4rem', alignItems: 'flex-start' }}>
                    <span style={{ flex: 1, color: '#a1a1aa', fontFamily: 'monospace', fontSize: '0.72rem', backgroundColor: '#09090b', padding: '0.35rem 0.5rem', borderRadius: '4px', wordBreak: 'break-all', lineHeight: 1.5 }}>
                      {displayHash}
                    </span>
                    <button
                      onClick={() => handleCopyHash(displayHash)}
                      title="Copy hash"
                      style={{ padding: '0.3rem 0.5rem', backgroundColor: copied ? 'rgba(16,185,129,0.15)' : '#27272a', color: copied ? '#10b981' : '#a1a1aa', border: '1px solid #3f3f46', borderRadius: '4px', cursor: 'pointer', fontSize: '0.78rem', flexShrink: 0 }}
                    >
                      {copied ? '✓' : '⎘'}
                    </button>
                  </div>
                ) : !isHashing && (
                  <span style={{ color: hashError ? '#f87171' : '#52525b', fontStyle: 'italic', fontSize: '0.82rem' }}>
                    {hashError ? `Error: ${hashError}` : fileDetail.currentlyMounted ? 'Computing...' : 'Unavailable (drive offline)'}
                  </span>
                )}
              </div>
            </div>

            {/* Duplicates */}
            <hr style={{ border: 0, borderTop: '1px solid #27272a', margin: 0 }} />

            <div>
              <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', marginBottom: '0.75rem' }}>
                <span style={{ color: '#71717a', fontSize: '0.78rem', textTransform: 'uppercase', letterSpacing: '0.05em' }}>Duplicates</span>
                {isDupeLoading && (
                  <span style={{ display: 'inline-block', width: '10px', height: '10px', border: '2px solid rgba(139,92,246,0.3)', borderTopColor: '#8b5cf6', borderRadius: '50%', animation: 'spin 0.7s linear infinite' }} />
                )}
              </div>

              {!isDupeLoading && duplicates && totalDupes === 0 && (
                <p style={{ margin: 0, color: '#52525b', fontSize: '0.83rem', fontStyle: 'italic' }}>No duplicates found in catalog</p>
              )}

              {/* Confirmed */}
              {duplicates && duplicates.confirmed.map(dupe => (
                <DupeCard key={dupe.id} dupe={dupe} />
              ))}

              {/* Probable */}
              {duplicates && duplicates.probable.length > 0 && (
                <>
                  <p style={{ margin: '0.75rem 0 0.4rem 0', color: '#71717a', fontSize: '0.76rem', textTransform: 'uppercase', letterSpacing: '0.05em' }}>
                    Probable (same name + size, not yet hashed)
                  </p>
                  {duplicates.probable.map(dupe => (
                    <DupeCard key={dupe.id} dupe={dupe} />
                  ))}
                </>
              )}
            </div>

            {/* Actions */}
            {fileDetail.currentlyMounted && fileDetail.currentPath && (
              <div style={{ display: 'flex', gap: '0.5rem' }}>
                <button
                  onClick={() => openPath(fileDetail.currentPath!)}
                  style={{ flex: 1, padding: '0.6rem', background: 'linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%)', color: 'white', border: 'none', borderRadius: '6px', fontWeight: 500, cursor: 'pointer', boxShadow: '0 4px 15px rgba(99,102,241,0.3)' }}
                  onMouseOver={e => e.currentTarget.style.opacity = '0.9'}
                  onMouseOut={e => e.currentTarget.style.opacity = '1'}
                >
                  Open File
                </button>
                <button
                  onClick={() => invoke('reveal_in_explorer', { path: fileDetail.currentPath! })}
                  style={{ flex: 1, padding: '0.6rem', background: '#27272a', color: 'white', border: '1px solid #3f3f46', borderRadius: '6px', fontWeight: 500, cursor: 'pointer' }}
                  onMouseOver={e => e.currentTarget.style.backgroundColor = '#3f3f46'}
                  onMouseOut={e => e.currentTarget.style.backgroundColor = '#27272a'}
                >
                  Show in Explorer
                </button>
              </div>
            )}
          </div>
        )}
      </div>

      <style>{`@keyframes spin { to { transform: rotate(360deg); } }`}</style>
    </div>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '0.15rem' }}>
      <span style={{ color: '#71717a' }}>{label}</span>
      <span style={{ color: '#e4e4e7' }}>{value}</span>
    </div>
  );
}

function DupeCard({ dupe }: { dupe: DuplicateEntry }) {
  const isConfirmed = dupe.confidence === 'confirmed';
  return (
    <div style={{
      backgroundColor: isConfirmed ? 'rgba(239,68,68,0.08)' : 'rgba(245,158,11,0.06)',
      border: `1px solid ${isConfirmed ? 'rgba(239,68,68,0.25)' : 'rgba(245,158,11,0.2)'}`,
      borderRadius: '6px', padding: '0.55rem 0.75rem', marginBottom: '0.4rem', fontSize: '0.81rem'
    }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.15rem' }}>
        <span style={{ color: isConfirmed ? '#fca5a5' : '#fcd34d', fontWeight: 600, wordBreak: 'break-word', flex: 1 }}>{dupe.file_name}</span>
        <span style={{ fontSize: '0.7rem', padding: '0.1rem 0.35rem', borderRadius: '3px', marginLeft: '0.5rem', flexShrink: 0, backgroundColor: isConfirmed ? 'rgba(239,68,68,0.2)' : 'rgba(245,158,11,0.15)', color: isConfirmed ? '#f87171' : '#f59e0b' }}>
          {isConfirmed ? '✓ hash match' : '~ probable'}
        </span>
      </div>
      <p style={{ margin: '0 0 0.1rem 0', color: '#71717a' }}>{dupe.source_name}</p>
      {dupe.current_path && (
        <p style={{ margin: 0, color: '#52525b', fontFamily: 'monospace', fontSize: '0.72rem', wordBreak: 'break-all' }}>{dupe.current_path}</p>
      )}
    </div>
  );
}
