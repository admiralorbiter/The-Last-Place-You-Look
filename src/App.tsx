import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useAppStore } from "./stores/appStore";
import { useNavigationStore, Page } from "./stores/navigationStore";
import { Sources } from "./pages/Sources";
import { Library } from "./pages/Library";
import { Duplicates } from "./pages/Duplicates";
import "./App.css";

interface AppInfo {
  version: string;
  db_status: string;
}

function App() {
  const { appVersion, dbStatus, setAppReady, setAppVersion, setDbStatus } = useAppStore();
  const { activePage, setActivePage } = useNavigationStore();

  useEffect(() => {
    const fetchInfo = async () => {
      try {
        const info = await invoke<AppInfo>("get_app_info");
        setAppReady(true);
        setAppVersion(info.version);
        setDbStatus(info.db_status);
      } catch (e) {
        console.error("Failed to get app info:", e);
      }
    };

    fetchInfo();
    const unlisten = listen("app://ready", fetchInfo);
    return () => {
      unlisten.then(f => f());
    };
  }, [setAppReady, setAppVersion, setDbStatus]);

  const renderPage = () => {
    switch (activePage) {
      case 'sources': return <Sources />;
      case 'library': return <Library />;
      case 'duplicates': return <Duplicates />;
      default: return <div style={{ padding: '2rem', color: '#888' }}>Not implemented yet.</div>;
    }
  };

  const NavItem = ({ page, label, icon }: { page: Page, label: string, icon: string }) => (
    <button
      onClick={() => setActivePage(page)}
      style={{
        display: 'flex', alignItems: 'center', gap: '0.8rem',
        padding: '0.8rem 1rem',
        background: activePage === page ? 'rgba(99, 102, 241, 0.1)' : 'transparent',
        color: activePage === page ? '#8b5cf6' : '#a1a1aa',
        border: 'none', borderRight: activePage === page ? '3px solid #6366f1' : '3px solid transparent',
        width: '100%', textAlign: 'left', cursor: 'pointer',
        fontSize: '1rem', fontWeight: activePage === page ? 600 : 400,
        transition: 'all 0.2s ease',
        borderTopRightRadius: 0, borderBottomRightRadius: 0
      }}
      onMouseOver={e => { if (activePage !== page) { e.currentTarget.style.color = '#e4e4e7'; e.currentTarget.style.background = 'rgba(255,255,255,0.03)'; } }}
      onMouseOut={e => { if (activePage !== page) { e.currentTarget.style.color = '#a1a1aa'; e.currentTarget.style.background = 'transparent'; } }}
    >
      <span>{icon}</span> {label}
    </button>
  );

  return (
    <div style={{ display: 'flex', height: '100vh', width: '100vw', background: '#09090b', overflow: 'hidden' }}>
      
      {/* Sidebar Navigation */}
      <nav style={{ width: '220px', background: '#18181b', borderRight: '1px solid #27272a', display: 'flex', flexDirection: 'column' }}>
        <div style={{ padding: '1.5rem', marginBottom: '1rem', borderBottom: '1px solid #27272a' }}>
          <h1 style={{ margin: 0, fontSize: '1.2rem', color: '#e0e0e0', fontWeight: 800, letterSpacing: '-0.5px' }}>
            The Last Place<br/><span style={{ color: '#8b5cf6' }}>You Look</span>
          </h1>
        </div>
        
        <div style={{ display: 'flex', flexDirection: 'column', gap: '0.2rem' }}>
          <NavItem page="home" label="Dashboard" icon="🏠" />
          <NavItem page="library" label="Library" icon="📚" />
          <NavItem page="duplicates" label="Duplicates" icon="👯" />
          <NavItem page="sources" label="Sources" icon="💾" />
          <NavItem page="settings" label="Settings" icon="⚙️" />
        </div>

        <div style={{ marginTop: 'auto', padding: '1rem', borderTop: '1px solid #27272a', fontSize: '0.75rem', color: '#555' }}>
          <div>v{appVersion || "..."}</div>
          <div style={{ color: dbStatus === "ok" ? '#10b981' : '#ef4444' }}>
            DB {dbStatus === "ok" ? "Connected" : "Error"}
          </div>
        </div>
      </nav>

      {/* Main Content Area */}
      <main style={{ flex: 1, height: '100%', overflow: 'auto', padding: '2rem' }}>
        {renderPage()}
      </main>
      
    </div>
  );
}

export default App;
