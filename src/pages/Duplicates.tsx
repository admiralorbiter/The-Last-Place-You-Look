import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { openPath } from '@tauri-apps/plugin-opener';

// Format bytes helper
const formatBytes = (bytes: number) => {
  if (!bytes) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
};

// Types matching Rust payload
interface DuplicateGroupMember {
  id: string;
  file_name: string;
  current_path: string | null;
  volume_relative_path: string;
  source_name: string;
  source_kind: string;
  size_bytes: number;
  preferred_copy: boolean;
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

export function Duplicates() {
  const [data, setData] = useState<DuplicateGroupsResult | null>(null);
  const [phase, setPhase] = useState<'idle' | 'loading' | 'done'>('idle');
  const [error, setError] = useState<string | null>(null);
  const [verifyingGrps, setVerifyingGrps] = useState<Set<string>>(new Set());
  const [justVerified, setJustVerified] = useState<{ id: string; success: boolean } | null>(null);
  const [elapsedMs, setElapsedMs] = useState(0);

  const fetchGroups = async () => {
    try {
      setError(null);
      setPhase('loading');
      const startTs = Date.now();

      // Tick a timer every 250ms so the user can see it's working
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

  useEffect(() => {
    fetchGroups();
  }, []);

  const handleVerify = async (group: DuplicateGroup) => {
    setVerifyingGrps(prev => new Set(prev).add(group.group_id));
    setJustVerified(null);
    try {
      const ids = group.members.map(m => m.id);
      const allMatch = await invoke<boolean>('verify_probable_group', { fileIds: ids });
      
      setJustVerified({ id: group.group_id, success: allMatch });
      
      if (!allMatch) {
         // Briefly show error then refresh
         setTimeout(() => {
            fetchGroups();
            setJustVerified(null);
         }, 4000);
      } else {
          // Immediately refresh to move it to confirmed
         await fetchGroups();
         setJustVerified(null);
      }

    } catch (e) {
      console.error("Verification failed", e);
      alert(`Verification failed: ${e}`);
    } finally {
      setVerifyingGrps(prev => {
        const next = new Set(prev);
        next.delete(group.group_id);
        return next;
      });
    }
  };

  const handlePin = async (group: DuplicateGroup, fileId: string) => {
    try {
      const allIds = group.members.map(m => m.id);
      await invoke('set_preferred_copy', { fileId, groupMemberIds: allIds });
      await fetchGroups();
    } catch (e) {
      alert(`Failed to pin: ${e}`);
    }
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
          {/* Animated progress pulsing bar */}
          <div style={{ height: '4px', backgroundColor: '#27272a', borderRadius: '4px', overflow: 'hidden' }}>
            <div style={{ height: '100%', width: '40%', background: 'linear-gradient(90deg, transparent, #8b5cf6, transparent)', animation: 'slide 1.5s ease-in-out infinite', borderRadius: '4px' }} />
          </div>
          <p style={{ margin: 0, color: '#52525b', fontSize: '0.8rem' }}>
            This is fast — 2 SQL queries regardless of catalog size.
          </p>
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
        <h2 style={{ margin: '0 0 1rem 0', color: '#f4f4f5' }}>Duplicate Library</h2>
        <div style={{ backgroundColor: 'rgba(248,113,113,0.1)', border: '1px solid rgba(248,113,113,0.2)', padding: '1rem', borderRadius: '8px', color: '#f87171' }}>
          Failed to load duplicates: {error}
        </div>
      </div>
    );
  }

  const { confirmed, probable, total_recoverable_bytes } = data!;
  const showEmptyState = confirmed.length === 0 && probable.length === 0;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', paddingBottom: '2rem' }}>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: '1rem', marginBottom: '1.5rem' }}>
        <h2 style={{ margin: 0, color: '#f4f4f5', fontWeight: 600, fontSize: '1.8rem', letterSpacing: '-0.5px' }}>Duplicate Review</h2>
        <span style={{ color: '#52525b', fontSize: '0.8rem' }}>analyzed in {(elapsedMs / 1000).toFixed(2)}s</span>
      </div>
      
      {/* Summary Bar */}
      <div style={{ 
        display: 'flex', alignItems: 'center', gap: '2rem', padding: '1.5rem', 
        backgroundColor: '#18181b', border: '1px solid #27272a', borderRadius: '12px', 
        marginBottom: '2rem', boxShadow: '0 4px 20px rgba(0,0,0,0.2)' 
      }}>
        <div style={{ flex: 1 }}>
          <p style={{ margin: 0, color: '#a1a1aa', fontSize: '0.85rem', textTransform: 'uppercase', letterSpacing: '0.05em' }}>Confirmed Wasted Space</p>
          <p style={{ margin: '0.2rem 0 0 0', color: '#10b981', fontSize: '1.8rem', fontWeight: 700 }}>
            {formatBytes(total_recoverable_bytes)}
          </p>
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
        
        {/* Confirmed */}
        {confirmed.length > 0 && (
          <div>
            <h3 style={{ margin: '0 0 1rem 0', color: '#f4f4f5', fontSize: '1.2rem', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
              <span style={{ display: 'inline-block', width: '8px', height: '8px', borderRadius: '50%', backgroundColor: '#ef4444' }}/>
              Confirmed Exact Duplicates
            </h3>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
              {confirmed.map(g => (
                <DuplicateGroupCard key={g.group_id} group={g} onPin={handlePin} onVerify={handleVerify} />
              ))}
            </div>
          </div>
        )}

        {/* Probable */}
        {probable.length > 0 && (
          <div>
            <h3 style={{ margin: '0 0 1rem 0', color: '#f4f4f5', fontSize: '1.2rem', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
              <span style={{ display: 'inline-block', width: '8px', height: '8px', borderRadius: '50%', backgroundColor: '#f59e0b' }}/>
              Probable Candidates <span style={{ fontSize: '0.85rem', color: '#71717a', fontWeight: 400 }}>(same name and size, unhashed)</span>
            </h3>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
              {probable.map(g => (
                <DuplicateGroupCard 
                  key={g.group_id} 
                  group={g} 
                  onPin={handlePin} 
                  onVerify={handleVerify} 
                  isVerifying={verifyingGrps.has(g.group_id)}
                  verificationResult={justVerified?.id === g.group_id ? justVerified.success : null}
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

function DuplicateGroupCard({ 
  group, 
  onPin, 
  onVerify, 
  isVerifying = false,
  verificationResult = null
}: { 
  group: DuplicateGroup; 
  onPin: (g: DuplicateGroup, id: string) => void;
  onVerify: (g: DuplicateGroup) => void;
  isVerifying?: boolean;
  verificationResult?: boolean | null;
}) {
  const isConfirmed = group.confidence === 'confirmed';
  
  return (
    <div style={{ 
      backgroundColor: '#18181b', border: '1px solid #27272a', borderRadius: '10px', 
      overflow: 'hidden', display: 'flex', flexDirection: 'column' 
    }}>
      {/* Header */}
      <div style={{ 
        padding: '1rem 1.25rem', borderBottom: '1px solid #27272a', 
        backgroundColor: isConfirmed ? 'rgba(239,68,68,0.03)' : 'rgba(245,158,11,0.03)',
        display: 'flex', justifyContent: 'space-between', alignItems: 'center'
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
          <span style={{ 
            fontSize: '0.7rem', fontWeight: 600, padding: '0.2rem 0.5rem', borderRadius: '4px', letterSpacing: '0.05em', textTransform: 'uppercase',
            backgroundColor: isConfirmed ? 'rgba(239,68,68,0.15)' : 'rgba(245,158,11,0.15)', 
            color: isConfirmed ? '#f87171' : '#fcd34d' 
          }}>
            {isConfirmed ? '✓ CONFIRMED' : '~ PROBABLE'}
          </span>
          <h4 style={{ margin: 0, color: '#e4e4e7', fontSize: '1.05rem', fontWeight: 600 }}>{group.file_name}</h4>
          <span style={{ color: '#71717a', fontSize: '0.9rem' }}>&bull; {group.members.length} copies &bull; {formatBytes(group.size_bytes)} each</span>
        </div>
        
        {/* Verify Action for Probable */}
        {!isConfirmed && (
          <div>
            {verificationResult === false ? (
               <span style={{ color: '#f87171', fontSize: '0.85rem', fontWeight: 500, backgroundColor: 'rgba(248,113,113,0.1)', padding: '0.4rem 0.8rem', borderRadius: '6px' }}>
                 ✕ Not a duplicate (files differ)
               </span>
            ) : (
                <button 
                onClick={() => onVerify(group)}
                disabled={isVerifying}
                style={{ 
                  padding: '0.4rem 1rem', backgroundColor: isVerifying ? 'transparent' : 'rgba(139,92,246,0.1)', 
                  color: '#8b5cf6', border: '1px solid rgba(139,92,246,0.4)', borderRadius: '6px', 
                  fontSize: '0.85rem', fontWeight: 600, cursor: isVerifying ? 'default' : 'pointer', display: 'flex', alignItems: 'center', gap: '0.5rem',
                  transition: 'all 0.2s'
                }}
                onMouseOver={e => { if(!isVerifying) e.currentTarget.style.backgroundColor = 'rgba(139,92,246,0.2)' }}
                onMouseOut={e => { if(!isVerifying) e.currentTarget.style.backgroundColor = 'rgba(139,92,246,0.1)' }}
              >
                {isVerifying ? (
                  <><span style={{ display: 'inline-block', width: '12px', height: '12px', border: '2px solid rgba(139,92,246,0.3)', borderTopColor: '#8b5cf6', borderRadius: '50%', animation: 'spin 0.7s linear infinite' }} /> Verifying...</>
                ) : 'Verify Hash'}
              </button>
            )}
          </div>
        )}
      </div>

      {/* Member List */}
      <div style={{ padding: '0.5rem 0' }}>
        {group.members.map(member => {
          const isRecommended = group.recommended_id === member.id;
          const isPinned = member.preferred_copy;
          
          return (
            <div key={member.id} style={{ 
              display: 'flex', alignItems: 'center', padding: '0.75rem 1.25rem', gap: '1rem',
              backgroundColor: isPinned ? 'rgba(16,185,129,0.05)' : 'transparent',
              borderLeft: isPinned ? '3px solid #10b981' : '3px solid transparent'
            }}>
              
              {/* Info */}
              <div style={{ flex: 1, overflow: 'hidden' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem', marginBottom: '0.2rem' }}>
                  <span style={{ 
                    padding: '0.1rem 0.4rem', backgroundColor: member.source_kind === 'removable' ? '#3f3f46' : '#27272a', 
                    borderRadius: '4px', fontSize: '0.7rem', color: '#a1a1aa', textTransform: 'uppercase' 
                  }}>
                    {member.source_kind === 'removable' ? 'USB/External' : 'Local Disk'}
                  </span>
                  <span style={{ color: '#e4e4e7', fontSize: '0.9rem', fontWeight: 500 }}>{member.source_name}</span>
                  {!member.current_path && (
                    <span style={{ color: '#f59e0b', fontSize: '0.75rem', fontStyle: 'italic' }}>(Offline)</span>
                  )}
                </div>
                <div style={{ color: '#71717a', fontFamily: 'monospace', fontSize: '0.8rem', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                  {member.volume_relative_path}
                </div>
              </div>

              {/* Badges & Actions */}
              <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
                {isRecommended && !isPinned && (
                  <span style={{ color: '#8b5cf6', fontSize: '0.75rem', fontWeight: 600, backgroundColor: 'rgba(139,92,246,0.1)', padding: '0.2rem 0.5rem', borderRadius: '4px' }}>
                    Suggested Keeper
                  </span>
                )}
                {isPinned && (
                  <span style={{ color: '#10b981', fontSize: '0.8rem', fontWeight: 600, display: 'flex', alignItems: 'center', gap: '0.3rem' }}>
                    <span style={{ fontSize: '1rem' }}>★</span> Pinned
                  </span>
                )}
                
                <button
                  onClick={() => onPin(group, member.id)}
                  style={{ 
                    background: 'transparent', border: '1px solid #3f3f46', color: '#a1a1aa', padding: '0.3rem 0.6rem', borderRadius: '4px', fontSize: '0.8rem', cursor: 'pointer',
                    display: isPinned ? 'none' : 'block'
                  }}
                  onMouseOver={e => { e.currentTarget.style.backgroundColor = '#27272a'; e.currentTarget.style.color = '#e4e4e7'; }}
                  onMouseOut={e => { e.currentTarget.style.backgroundColor = 'transparent'; e.currentTarget.style.color = '#a1a1aa'; }}
                >
                  Pin
                </button>

                {member.current_path && (
                  <button
                    onClick={() => openPath(member.current_path!)}
                    style={{ background: 'transparent', border: 'none', color: '#3b82f6', fontSize: '0.8rem', cursor: 'pointer', padding: '0.3rem', textDecoration: 'underline' }}
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
