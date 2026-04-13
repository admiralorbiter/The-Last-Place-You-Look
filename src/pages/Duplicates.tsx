import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { openPath } from '@tauri-apps/plugin-opener';

const formatBytes = (bytes: number) => {
  if (!bytes) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
};

// Split a relative path into folder segments (strips filename)
const folderSegments = (volumeRelativePath: string): string[] => {
  const sep = volumeRelativePath.includes('\\') ? '\\' : '/';
  const parts = volumeRelativePath.split(sep);
  parts.pop(); // remove filename
  return parts.filter(Boolean);
};

// Build the prefix string up to and including segment at index `depth`
const buildPrefix = (segments: string[], depth: number, sep: string): string =>
  segments.slice(0, depth + 1).join(sep);

interface DuplicateGroupMember {
  id: string;
  file_name: string;
  current_path: string | null;
  volume_relative_path: string;
  source_name: string;
  source_id: string;
  source_kind: string;
  size_bytes: number;
  preferred_copy: boolean;
  is_intentional_backup: boolean;
}

interface DuplicateGroup {
  group_id: string;
  confidence: 'confirmed' | 'probable';
  file_name: string;
  size_bytes: number;
  members: DuplicateGroupMember[];
  recommended_id: string | null;
}

interface DuplicateGroupsResult {
  confirmed: DuplicateGroup[];
  probable: DuplicateGroup[];
  total_recoverable_bytes: number;
}

interface ExcludedPath {
  id: string;
  source_id: string | null;
  source_name: string | null;
  volume_path_prefix: string;
  pattern_type: string;  // 'folder' | 'file_name' | 'extension'
  label: string | null;
}

// ── Ignore Panel ─────────────────────────────────────────────────────────────
// Three modes: ignore by filename, extension, or folder prefix.
// Folder mode uses a clickable breadcrumb picker; file/ext modes are one-click.
type IgnoreMode = 'file_name' | 'extension' | 'folder';

function IgnorePanel({
  group,
  onConfirm,
  onCancel,
}: {
  group: DuplicateGroup;
  onConfirm: (sourceId: string | null, value: string, patternType: string, label: string) => void;
  onCancel: () => void;
}) {
  const [mode, setMode] = useState<IgnoreMode>('file_name');
  const [basisIdx, setBasisIdx] = useState(0);
  const [selectedDepth, setSelectedDepth] = useState<number | null>(null);

  const basis = group.members[basisIdx];
  const sep = basis.volume_relative_path.includes('\\') ? '\\' : '/';
  const segments = folderSegments(basis.volume_relative_path);

  // Derive extension from the group filename
  const fileName = group.file_name;
  const extMatch = fileName.match(/(\.[^.]+)$/);
  const extension = extMatch ? extMatch[1] : null;

  const selectedFolderPrefix = mode === 'folder' && selectedDepth !== null
    ? buildPrefix(segments, selectedDepth, sep)
    : null;

  const tabStyle = (active: boolean): React.CSSProperties => ({
    padding: '0.3rem 0.75rem', borderRadius: '5px', fontSize: '0.8rem', cursor: 'pointer',
    border: active ? '1px solid rgba(245,158,11,0.5)' : '1px solid #3f3f46',
    backgroundColor: active ? 'rgba(245,158,11,0.15)' : 'transparent',
    color: active ? '#fbbf24' : '#71717a', fontWeight: active ? 600 : 400,
    transition: 'all 0.15s',
  });

  return (
    <div style={{
      padding: '1rem 1.25rem', borderTop: '1px solid rgba(245,158,11,0.2)',
      backgroundColor: 'rgba(245,158,11,0.02)', display: 'flex', flexDirection: 'column', gap: '0.75rem'
    }}>
      {/* Mode tabs */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
        <span style={{ color: '#71717a', fontSize: '0.8rem', marginRight: '0.25rem' }}>Ignore by:</span>
        <button style={tabStyle(mode === 'file_name')} onClick={() => { setMode('file_name'); setSelectedDepth(null); }}>
          📄 Filename
        </button>
        {extension && (
          <button style={tabStyle(mode === 'extension')} onClick={() => { setMode('extension'); setSelectedDepth(null); }}>
            🔤 Extension
          </button>
        )}
        <button style={tabStyle(mode === 'folder')} onClick={() => { setMode('folder'); setSelectedDepth(null); }}>
          📁 Folder
        </button>
      </div>

      {/* File name mode */}
      {mode === 'file_name' && (
        <div style={{ display: 'flex', alignItems: 'center', gap: '1rem', flexWrap: 'wrap' }}>
          <span style={{ color: '#a1a1aa', fontSize: '0.82rem' }}>
            Suppress all files named exactly{' '}
            <code style={{ color: '#fcd34d', backgroundColor: 'rgba(245,158,11,0.08)', padding: '0.1rem 0.4rem', borderRadius: '3px' }}>
              {fileName}
            </code>{' '}
            <span style={{ color: '#52525b' }}>(globally, all sources)</span>
          </span>
          <button
            onClick={() => onConfirm(null, fileName, 'file_name', fileName)}
            style={{ padding: '0.35rem 0.9rem', backgroundColor: 'rgba(245,158,11,0.15)', color: '#fbbf24', border: '1px solid rgba(245,158,11,0.4)', borderRadius: '6px', fontSize: '0.82rem', fontWeight: 600, cursor: 'pointer' }}
          >
            ⊘ Ignore this filename
          </button>
          <button onClick={onCancel} style={cancelBtnStyle}>Cancel</button>
        </div>
      )}

      {/* Extension mode */}
      {mode === 'extension' && extension && (
        <div style={{ display: 'flex', alignItems: 'center', gap: '1rem', flexWrap: 'wrap' }}>
          <span style={{ color: '#a1a1aa', fontSize: '0.82rem' }}>
            Suppress all{' '}
            <code style={{ color: '#fcd34d', backgroundColor: 'rgba(245,158,11,0.08)', padding: '0.1rem 0.4rem', borderRadius: '3px' }}>
              {extension}
            </code>{' '}
            files{' '}
            <span style={{ color: '#52525b' }}>(globally, all sources)</span>
          </span>
          <button
            onClick={() => onConfirm(null, extension, 'extension', extension)}
            style={{ padding: '0.35rem 0.9rem', backgroundColor: 'rgba(245,158,11,0.15)', color: '#fbbf24', border: '1px solid rgba(245,158,11,0.4)', borderRadius: '6px', fontSize: '0.82rem', fontWeight: 600, cursor: 'pointer' }}
          >
            ⊘ Ignore all {extension} files
          </button>
          <button onClick={onCancel} style={cancelBtnStyle}>Cancel</button>
        </div>
      )}

      {/* Folder mode — breadcrumb picker */}
      {mode === 'folder' && (
        <>
          {/* Source picker for cross-source groups */}
          {group.members.length > 1 && (
            <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem', fontSize: '0.82rem' }}>
              <span style={{ color: '#71717a' }}>Based on path from:</span>
              <select
                value={basisIdx}
                onChange={e => { setBasisIdx(Number(e.target.value)); setSelectedDepth(null); }}
                style={{ background: '#27272a', border: '1px solid #3f3f46', color: '#e4e4e7', borderRadius: '4px', padding: '0.2rem 0.4rem', fontSize: '0.82rem' }}
              >
                {group.members.map((m, i) => (
                  <option key={m.id} value={i}>{m.source_name} — {m.volume_relative_path}</option>
                ))}
              </select>
            </div>
          )}

          {segments.length === 0 ? (
            <span style={{ color: '#52525b', fontSize: '0.82rem', fontStyle: 'italic' }}>
              This file is in the source root — no parent folder to exclude.
            </span>
          ) : (
            <>
              <p style={{ margin: 0, color: '#a1a1aa', fontSize: '0.82rem' }}>
                Click a segment to choose the exclusion depth on <strong style={{ color: '#e4e4e7' }}>{basis.source_name}</strong>:
              </p>
              <div style={{ display: 'flex', alignItems: 'center', flexWrap: 'wrap', gap: '2px' }}>
                {segments.map((seg, idx) => {
                  const isSelected = selectedDepth !== null && idx <= selectedDepth;
                  const isTarget = idx === selectedDepth;
                  return (
                    <React.Fragment key={idx}>
                      {idx > 0 && <span style={{ color: '#3f3f46', fontSize: '0.8rem', padding: '0 1px' }}>{sep}</span>}
                      <button
                        onClick={() => setSelectedDepth(idx)}
                        style={{
                          background: isSelected ? (isTarget ? 'rgba(245,158,11,0.25)' : 'rgba(245,158,11,0.12)') : 'rgba(255,255,255,0.04)',
                          border: `1px solid ${isSelected ? 'rgba(245,158,11,0.5)' : '#3f3f46'}`,
                          color: isSelected ? '#fcd34d' : '#a1a1aa',
                          borderRadius: '4px', padding: '0.2rem 0.5rem', fontSize: '0.78rem',
                          cursor: 'pointer', fontFamily: 'monospace', transition: 'all 0.15s',
                        }}
                      >
                        {seg}
                      </button>
                    </React.Fragment>
                  );
                })}
              </div>

              {selectedFolderPrefix ? (
                <div style={{ display: 'flex', alignItems: 'center', gap: '1rem', flexWrap: 'wrap' }}>
                  <span style={{ color: '#71717a', fontSize: '0.82rem' }}>
                    Will exclude:{' '}
                    <code style={{ color: '#fcd34d', backgroundColor: 'rgba(245,158,11,0.08)', padding: '0.1rem 0.3rem', borderRadius: '3px' }}>
                      {selectedFolderPrefix}
                    </code>{' '}
                    on {basis.source_name}
                  </span>
                  <button
                    onClick={() => onConfirm(basis.source_id, selectedFolderPrefix, 'folder', segments[selectedDepth!])}
                    style={{ padding: '0.35rem 0.9rem', backgroundColor: 'rgba(245,158,11,0.15)', color: '#fbbf24', border: '1px solid rgba(245,158,11,0.4)', borderRadius: '6px', fontSize: '0.82rem', fontWeight: 600, cursor: 'pointer' }}
                  >
                    ⊘ Exclude this folder
                  </button>
                  <button onClick={onCancel} style={cancelBtnStyle}>Cancel</button>
                </div>
              ) : (
                <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
                  <span style={{ color: '#52525b', fontSize: '0.8rem', fontStyle: 'italic' }}>← Click a folder segment above</span>
                  <button onClick={onCancel} style={cancelBtnStyle}>Cancel</button>
                </div>
              )}
            </>
          )}
        </>
      )}
    </div>
  );
}

const cancelBtnStyle: React.CSSProperties = {
  background: 'transparent', border: '1px solid #3f3f46', color: '#71717a',
  padding: '0.3rem 0.7rem', borderRadius: '4px', fontSize: '0.8rem', cursor: 'pointer'
};

// ── Main Page ───────────────────────────────────────────────────────────────
export function Duplicates() {
  const [data, setData] = useState<DuplicateGroupsResult | null>(null);
  const [phase, setPhase] = useState<'idle' | 'loading' | 'done'>('idle');
  const [error, setError] = useState<string | null>(null);
  const [verifyingGrps, setVerifyingGrps] = useState<Set<string>>(new Set());
  const [justVerified, setJustVerified] = useState<{ id: string; success: boolean } | null>(null);
  const [elapsedMs, setElapsedMs] = useState(0);
  const [exclusions, setExclusions] = useState<ExcludedPath[]>([]);
  const [showExclusions, setShowExclusions] = useState(false);
  const [ignoringGroupId, setIgnoringGroupId] = useState<string | null>(null);
  const [toast, setToast] = useState<string | null>(null);

  const showToast = (msg: string) => {
    setToast(msg);
    setTimeout(() => setToast(null), 3500);
  };

  const fetchGroups = async () => {
    try {
      setError(null);
      setPhase('loading');
      const startTs = Date.now();
      const timer = setInterval(() => setElapsedMs(Date.now() - startTs), 250);
      const result = await invoke<DuplicateGroupsResult>('list_duplicate_groups');
      clearInterval(timer);
      setElapsedMs(Date.now() - startTs);
      setData(result);
      setPhase('done');
    } catch (e) {
      setError(String(e));
      setPhase('done');
    }
  };

  const fetchExclusions = async () => {
    try {
      setExclusions(await invoke<ExcludedPath[]>('list_excluded_paths'));
    } catch (_) {}
  };

  useEffect(() => { fetchGroups(); fetchExclusions(); }, []);

  const handleVerify = async (group: DuplicateGroup) => {
    setVerifyingGrps(prev => new Set(prev).add(group.group_id));
    setJustVerified(null);
    try {
      const ids = group.members.map(m => m.id);
      const allMatch = await invoke<boolean>('verify_probable_group', { fileIds: ids });
      setJustVerified({ id: group.group_id, success: allMatch });
      if (!allMatch) {
        setTimeout(() => { fetchGroups(); setJustVerified(null); }, 4000);
      } else {
        await fetchGroups(); setJustVerified(null);
      }
    } catch (e) { alert(`Verification failed: ${e}`); }
    finally {
      setVerifyingGrps(prev => { const n = new Set(prev); n.delete(group.group_id); return n; });
    }
  };

  const handlePin = async (group: DuplicateGroup, fileId: string) => {
    try {
      await invoke('set_preferred_copy', { fileId, groupMemberIds: group.members.map(m => m.id) });
      await fetchGroups();
    } catch (e) { alert(`Failed to pin: ${e}`); }
  };

  const handleToggleBackup = async (fileId: string, current: boolean) => {
    try {
      await invoke('set_intentional_backup', { fileId, isBackup: !current });
      await fetchGroups();
    } catch (e) { alert(`Failed to update backup status: ${e}`); }
  };

  const handleIgnoreConfirm = async (sourceId: string | null, value: string, patternType: string, label: string) => {
    try {
      await invoke('add_excluded_path', { sourceId, volumePathPrefix: value, patternType, label });
      setIgnoringGroupId(null);
      showToast(`"${label}" excluded — refreshing...`);
      await fetchGroups();
      await fetchExclusions();
    } catch (e) { alert(`Failed to add exclusion: ${e}`); }
  };

  const handleRemoveExclusion = async (id: string) => {
    try {
      await invoke('remove_excluded_path', { id });
      await fetchExclusions();
      await fetchGroups();
    } catch (e) { alert(`Failed to remove exclusion: ${e}`); }
  };

  if (phase === 'idle' || phase === 'loading') {
    return (
      <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
        <h2 style={{ margin: '0 0 1.5rem 0', color: '#f4f4f5', fontWeight: 600, fontSize: '1.8rem', letterSpacing: '-0.5px' }}>Duplicate Review</h2>
        <div style={{ backgroundColor: '#18181b', border: '1px solid #27272a', borderRadius: '12px', padding: '2rem', display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
            <span style={{ display: 'inline-block', width: '20px', height: '20px', border: '3px solid rgba(139,92,246,0.3)', borderTopColor: '#8b5cf6', borderRadius: '50%', animation: 'spin 0.8s linear infinite', flexShrink: 0 }} />
            <div>
              <p style={{ margin: 0, color: '#e4e4e7', fontWeight: 500 }}>Analyzing catalog...</p>
              <p style={{ margin: '0.2rem 0 0', color: '#71717a', fontSize: '0.85rem' }}>
                Scanning ≥512KB files for probable matches — {(elapsedMs / 1000).toFixed(1)}s elapsed
              </p>
            </div>
          </div>
          <div style={{ height: '4px', backgroundColor: '#27272a', borderRadius: '4px', overflow: 'hidden' }}>
            <div style={{ height: '100%', width: '40%', background: 'linear-gradient(90deg, transparent, #8b5cf6, transparent)', animation: 'slide 1.5s ease-in-out infinite', borderRadius: '4px' }} />
          </div>
        </div>
        <style>{`
          @keyframes spin { to { transform: rotate(360deg); } }
          @keyframes slide { 0% { transform: translateX(-200%); } 100% { transform: translateX(400%); } }
        `}</style>
      </div>
    );
  }

  if (error) {
    return (
      <div>
        <h2 style={{ margin: '0 0 1rem 0', color: '#f4f4f5' }}>Duplicate Review</h2>
        <div style={{ backgroundColor: 'rgba(248,113,113,0.1)', border: '1px solid rgba(248,113,113,0.2)', padding: '1rem', borderRadius: '8px', color: '#f87171' }}>
          Failed to load duplicates: {error}
        </div>
      </div>
    );
  }

  const { confirmed, probable, total_recoverable_bytes } = data!;
  const showEmptyState = confirmed.length === 0 && probable.length === 0;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', paddingBottom: '2rem', position: 'relative' }}>

      {/* Toast */}
      {toast && (
        <div style={{ position: 'fixed', bottom: '2rem', right: '2rem', backgroundColor: '#18181b', border: '1px solid #3f3f46', borderRadius: '8px', padding: '0.75rem 1.25rem', color: '#e4e4e7', fontSize: '0.9rem', zIndex: 9999, boxShadow: '0 4px 20px rgba(0,0,0,0.4)' }}>
          {toast}
        </div>
      )}

      {/* Page Header */}
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '1.5rem' }}>
        <div style={{ display: 'flex', alignItems: 'baseline', gap: '1rem' }}>
          <h2 style={{ margin: 0, color: '#f4f4f5', fontWeight: 600, fontSize: '1.8rem', letterSpacing: '-0.5px' }}>Duplicate Review</h2>
          <span style={{ color: '#52525b', fontSize: '0.8rem' }}>analyzed in {(elapsedMs / 1000).toFixed(2)}s</span>
        </div>
        <button
          onClick={() => setShowExclusions(v => !v)}
          style={{
            background: showExclusions ? 'rgba(245,158,11,0.1)' : 'transparent',
            border: `1px solid ${exclusions.length > 0 ? 'rgba(245,158,11,0.4)' : '#3f3f46'}`,
            color: exclusions.length > 0 ? '#f59e0b' : '#71717a',
            padding: '0.35rem 0.85rem', borderRadius: '6px', fontSize: '0.8rem', cursor: 'pointer'
          }}
        >
          ⊘ Excluded Folders {exclusions.length > 0 ? `(${exclusions.length})` : ''}
        </button>
      </div>

      {/* Excluded Folders Panel */}
      {showExclusions && (
        <div style={{ backgroundColor: '#18181b', border: '1px solid #3f3f46', borderRadius: '10px', padding: '1.25rem', marginBottom: '1.5rem' }}>
          <h4 style={{ margin: '0 0 0.75rem 0', color: '#e4e4e7', fontSize: '0.95rem' }}>⊘ Active Exclusions</h4>
          {exclusions.length === 0 ? (
            <p style={{ margin: 0, color: '#52525b', fontSize: '0.85rem' }}>
              No folders excluded yet. Use the <strong>⊘ Ignore folder</strong> button on any duplicate group to suppress it.
            </p>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
              {exclusions.map(ex => (
                <div key={ex.id} style={{ display: 'flex', alignItems: 'center', gap: '0.75rem', padding: '0.5rem 0.75rem', backgroundColor: '#27272a', borderRadius: '6px' }}>
                  <span style={{
                    fontSize: '0.7rem', padding: '0.1rem 0.4rem', borderRadius: '3px', flexShrink: 0,
                    backgroundColor: ex.pattern_type === 'folder' ? 'rgba(139,92,246,0.15)' : ex.pattern_type === 'extension' ? 'rgba(59,130,246,0.15)' : 'rgba(16,185,129,0.15)',
                    color: ex.pattern_type === 'folder' ? '#a78bfa' : ex.pattern_type === 'extension' ? '#60a5fa' : '#34d399',
                  }}>
                    {ex.pattern_type === 'folder' ? '📁 Folder' : ex.pattern_type === 'extension' ? '🔤 Ext' : '📄 File'}
                  </span>
                  <div style={{ flex: 1 }}>
                    <span style={{ color: '#fcd34d', fontSize: '0.85rem', fontFamily: 'monospace' }}>{ex.volume_path_prefix}</span>
                    {ex.source_name && <span style={{ color: '#71717a', fontSize: '0.8rem' }}> on {ex.source_name}</span>}
                    {!ex.source_name && ex.pattern_type !== 'folder' && <span style={{ color: '#52525b', fontSize: '0.8rem' }}> (all sources)</span>}
                  </div>
                  <button
                    onClick={() => handleRemoveExclusion(ex.id)}
                    style={{ background: 'transparent', border: '1px solid rgba(248,113,113,0.3)', color: '#f87171', padding: '0.2rem 0.6rem', borderRadius: '4px', fontSize: '0.75rem', cursor: 'pointer', flexShrink: 0 }}
                  >
                    Remove
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Summary Bar */}
      <div style={{
        display: 'flex', alignItems: 'center', gap: '2rem', padding: '1.5rem',
        backgroundColor: '#18181b', border: '1px solid #27272a', borderRadius: '12px',
        marginBottom: '2rem', boxShadow: '0 4px 20px rgba(0,0,0,0.2)'
      }}>
        <div style={{ flex: 1 }}>
          <p style={{ margin: 0, color: '#a1a1aa', fontSize: '0.85rem', textTransform: 'uppercase', letterSpacing: '0.05em' }}>Confirmed Wasted Space</p>
          <p style={{ margin: '0.2rem 0 0 0', color: '#10b981', fontSize: '1.8rem', fontWeight: 700 }}>{formatBytes(total_recoverable_bytes)}</p>
        </div>
        <div style={{ paddingLeft: '2rem', borderLeft: '1px solid #27272a' }}>
          <p style={{ margin: 0, color: '#a1a1aa', fontSize: '0.85rem', textTransform: 'uppercase', letterSpacing: '0.05em' }}>Confirmed Groups</p>
          <p style={{ margin: '0.2rem 0 0 0', color: '#e4e4e7', fontSize: '1.4rem', fontWeight: 600 }}>{confirmed.length}</p>
        </div>
        <div style={{ paddingLeft: '2rem', borderLeft: '1px solid #27272a' }}>
          <p style={{ margin: 0, color: '#a1a1aa', fontSize: '0.85rem', textTransform: 'uppercase', letterSpacing: '0.05em' }}>Probable Groups</p>
          <p style={{ margin: '0.2rem 0 0 0', color: '#e4e4e7', fontSize: '1.4rem', fontWeight: 600 }}>{probable.length}</p>
        </div>
      </div>

      {showEmptyState && (
        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', padding: '4rem 2rem', backgroundColor: '#18181b', border: '1px dashed #3f3f46', borderRadius: '12px' }}>
          <span style={{ fontSize: '3rem', marginBottom: '1rem' }}>✨</span>
          <h3 style={{ margin: '0 0 0.5rem 0', color: '#e4e4e7' }}>No duplicates found</h3>
          <p style={{ margin: 0, color: '#a1a1aa', textAlign: 'center' }}>Your catalog looks fully deduplicated.</p>
        </div>
      )}

      {/* Group Lists */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: '2rem' }}>
        {confirmed.length > 0 && (
          <div>
            <h3 style={{ margin: '0 0 1rem 0', color: '#f4f4f5', fontSize: '1.2rem', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
              <span style={{ display: 'inline-block', width: '8px', height: '8px', borderRadius: '50%', backgroundColor: '#ef4444' }}/>
              Confirmed Exact Duplicates
            </h3>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
              {confirmed.map(g => (
                <DuplicateGroupCard
                  key={g.group_id} group={g}
                  onPin={handlePin} onVerify={handleVerify} onToggleBackup={handleToggleBackup}
                  isIgnoring={ignoringGroupId === g.group_id}
                  onIgnoreOpen={() => setIgnoringGroupId(g.group_id)}
                  onIgnoreCancel={() => setIgnoringGroupId(null)}
                  onIgnoreConfirm={handleIgnoreConfirm}
                />
              ))}
            </div>
          </div>
        )}

        {probable.length > 0 && (
          <div>
            <h3 style={{ margin: '0 0 1rem 0', color: '#f4f4f5', fontSize: '1.2rem', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
              <span style={{ display: 'inline-block', width: '8px', height: '8px', borderRadius: '50%', backgroundColor: '#f59e0b' }}/>
              Probable Candidates <span style={{ fontSize: '0.85rem', color: '#71717a', fontWeight: 400 }}>(same name and size, unhashed)</span>
            </h3>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
              {probable.map(g => (
                <DuplicateGroupCard
                  key={g.group_id} group={g}
                  onPin={handlePin} onVerify={handleVerify} onToggleBackup={handleToggleBackup}
                  isVerifying={verifyingGrps.has(g.group_id)}
                  verificationResult={justVerified?.id === g.group_id ? justVerified.success : null}
                  isIgnoring={ignoringGroupId === g.group_id}
                  onIgnoreOpen={() => setIgnoringGroupId(g.group_id)}
                  onIgnoreCancel={() => setIgnoringGroupId(null)}
                  onIgnoreConfirm={handleIgnoreConfirm}
                />
              ))}
            </div>
          </div>
        )}
      </div>
      <style>{`@keyframes spin { to { transform: rotate(360deg); } }`}</style>
    </div>
  );
}

// ── Group Card ───────────────────────────────────────────────────────────────
function DuplicateGroupCard({
  group, onPin, onVerify, onToggleBackup,
  isVerifying = false, verificationResult = null,
  isIgnoring, onIgnoreOpen, onIgnoreCancel, onIgnoreConfirm,
}: {
  group: DuplicateGroup;
  onPin: (g: DuplicateGroup, id: string) => void;
  onVerify: (g: DuplicateGroup) => void;
  onToggleBackup: (fileId: string, current: boolean) => void;
  isVerifying?: boolean;
  verificationResult?: boolean | null;
  isIgnoring: boolean;
  onIgnoreOpen: () => void;
  onIgnoreCancel: () => void;
  onIgnoreConfirm: (sourceId: string | null, value: string, patternType: string, label: string) => void;
}) {
  const isConfirmed = group.confidence === 'confirmed';

  return (
    <div style={{
      backgroundColor: '#18181b', border: `1px solid ${isIgnoring ? 'rgba(245,158,11,0.35)' : '#27272a'}`,
      borderRadius: '10px', overflow: 'hidden', display: 'flex', flexDirection: 'column',
      transition: 'border-color 0.2s'
    }}>
      {/* Header */}
      <div style={{
        padding: '0.9rem 1.25rem', borderBottom: isIgnoring ? '1px solid rgba(245,158,11,0.2)' : '1px solid #27272a',
        backgroundColor: isConfirmed ? 'rgba(239,68,68,0.03)' : 'rgba(245,158,11,0.03)',
        display: 'flex', justifyContent: 'space-between', alignItems: 'center', gap: '1rem'
      }}>
        {/* Left: confidence badge + name + count */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem', overflow: 'hidden' }}>
          <span style={{
            fontSize: '0.65rem', fontWeight: 700, padding: '0.2rem 0.5rem', borderRadius: '4px',
            letterSpacing: '0.07em', textTransform: 'uppercase', flexShrink: 0,
            backgroundColor: isConfirmed ? 'rgba(239,68,68,0.15)' : 'rgba(245,158,11,0.15)',
            color: isConfirmed ? '#f87171' : '#fcd34d'
          }}>
            {isConfirmed ? '✓ CONFIRMED' : '~ PROBABLE'}
          </span>
          <h4 style={{ margin: 0, color: '#e4e4e7', fontSize: '1rem', fontWeight: 600, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
            {group.file_name}
          </h4>
          <span style={{ color: '#71717a', fontSize: '0.85rem', whiteSpace: 'nowrap', flexShrink: 0 }}>
            &bull; {group.members.length} copies &bull; {formatBytes(group.size_bytes)} each
          </span>
        </div>

        {/* Right: actions */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', flexShrink: 0 }}>
          {/* Verify Hash (probable only) */}
          {!isConfirmed && (
            verificationResult === false ? (
              <span style={{ color: '#f87171', fontSize: '0.8rem', backgroundColor: 'rgba(248,113,113,0.1)', padding: '0.3rem 0.7rem', borderRadius: '6px', fontWeight: 500 }}>
                ✕ Files differ
              </span>
            ) : (
              <button
                onClick={() => onVerify(group)} disabled={isVerifying}
                style={{
                  padding: '0.35rem 0.85rem', backgroundColor: 'rgba(139,92,246,0.1)',
                  color: '#8b5cf6', border: '1px solid rgba(139,92,246,0.35)', borderRadius: '6px',
                  fontSize: '0.8rem', fontWeight: 600, cursor: 'pointer', display: 'flex', alignItems: 'center', gap: '0.4rem'
                }}
              >
                {isVerifying
                  ? <><span style={{ display: 'inline-block', width: '11px', height: '11px', border: '2px solid rgba(139,92,246,0.3)', borderTopColor: '#8b5cf6', borderRadius: '50%', animation: 'spin 0.7s linear infinite' }} /> Verifying...</>
                  : 'Verify Hash'
                }
              </button>
            )
          )}

          {/* Ignore folder */}
          <button
            onClick={isIgnoring ? onIgnoreCancel : onIgnoreOpen}
            style={{
              padding: '0.35rem 0.85rem',
              backgroundColor: isIgnoring ? 'rgba(245,158,11,0.15)' : 'transparent',
              color: isIgnoring ? '#fbbf24' : '#71717a',
              border: `1px solid ${isIgnoring ? 'rgba(245,158,11,0.45)' : '#3f3f46'}`,
              borderRadius: '6px', fontSize: '0.8rem', cursor: 'pointer', fontWeight: isIgnoring ? 600 : 400,
              transition: 'all 0.15s'
            }}
          >
            {isIgnoring ? '✕ Cancel' : '⊘ Ignore folder'}
          </button>
        </div>
      </div>

      {/* Inline ignore panel */}
      {isIgnoring && (
        <IgnorePanel
          group={group}
          onConfirm={onIgnoreConfirm}
          onCancel={onIgnoreCancel}
        />
      )}

      {/* Member rows */}
      <div style={{ padding: '0.25rem 0' }}>
        {group.members.map(member => {
          const isRecommended = group.recommended_id === member.id;
          const isPinned = member.preferred_copy;
          const isBackup = member.is_intentional_backup;

          return (
            <div key={member.id} style={{
              display: 'flex', alignItems: 'center', padding: '0.7rem 1.25rem', gap: '1rem',
              backgroundColor: isPinned ? 'rgba(16,185,129,0.05)' : isBackup ? 'rgba(59,130,246,0.04)' : 'transparent',
              borderLeft: `3px solid ${isPinned ? '#10b981' : isBackup ? '#3b82f6' : 'transparent'}`
            }}>

              {/* File info */}
              <div style={{ flex: 1, overflow: 'hidden' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '0.6rem', marginBottom: '0.2rem' }}>
                  <span style={{
                    padding: '0.1rem 0.35rem', borderRadius: '3px', fontSize: '0.68rem',
                    color: '#a1a1aa', textTransform: 'uppercase',
                    backgroundColor: member.source_kind === 'removable' ? '#3f3f46' : '#27272a'
                  }}>
                    {member.source_kind === 'removable' ? 'USB/EXT' : 'LOCAL'}
                  </span>
                  <span style={{ color: '#e4e4e7', fontSize: '0.88rem', fontWeight: 500 }}>{member.source_name}</span>
                  {!member.current_path && <span style={{ color: '#f59e0b', fontSize: '0.72rem', fontStyle: 'italic' }}>(Offline)</span>}
                </div>
                <div style={{ color: '#71717a', fontFamily: 'monospace', fontSize: '0.78rem', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                  {member.volume_relative_path}
                </div>
              </div>

              {/* Badges + actions */}
              <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', flexShrink: 0 }}>
                {isBackup && (
                  <span style={{ color: '#60a5fa', fontSize: '0.72rem', fontWeight: 600, backgroundColor: 'rgba(59,130,246,0.12)', padding: '0.15rem 0.45rem', borderRadius: '4px' }}>
                    ☁ Backup
                  </span>
                )}
                {isRecommended && !isPinned && !isBackup && (
                  <span style={{ color: '#8b5cf6', fontSize: '0.72rem', fontWeight: 600, backgroundColor: 'rgba(139,92,246,0.1)', padding: '0.15rem 0.45rem', borderRadius: '4px' }}>
                    Suggested Keeper
                  </span>
                )}
                {isPinned && (
                  <span style={{ color: '#10b981', fontSize: '0.78rem', fontWeight: 600 }}>★ Pinned</span>
                )}

                {/* ☁ Backup toggle */}
                <button
                  onClick={() => onToggleBackup(member.id, isBackup)}
                  title={isBackup ? 'Unmark as intentional backup' : 'Mark as intentional backup'}
                  style={{
                    background: isBackup ? 'rgba(59,130,246,0.12)' : 'transparent',
                    border: `1px solid ${isBackup ? 'rgba(59,130,246,0.4)' : '#3f3f46'}`,
                    color: isBackup ? '#60a5fa' : '#52525b',
                    padding: '0.25rem 0.45rem', borderRadius: '4px', fontSize: '0.8rem', cursor: 'pointer'
                  }}
                >
                  ☁
                </button>

                {/* Pin */}
                {!isPinned && (
                  <button
                    onClick={() => onPin(group, member.id)}
                    style={{ background: 'transparent', border: '1px solid #3f3f46', color: '#a1a1aa', padding: '0.25rem 0.55rem', borderRadius: '4px', fontSize: '0.78rem', cursor: 'pointer' }}
                  >
                    Pin
                  </button>
                )}

                {/* Open */}
                {member.current_path && (
                  <button
                    onClick={() => openPath(member.current_path!)}
                    style={{ background: 'transparent', border: 'none', color: '#3b82f6', fontSize: '0.78rem', cursor: 'pointer', textDecoration: 'underline', padding: '0.25rem' }}
                  >
                    Open
                  </button>
                )}
              </div>

            </div>
          );
        })}
      </div>
    </div>
  );
}
