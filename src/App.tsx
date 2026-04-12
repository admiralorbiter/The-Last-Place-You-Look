import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useAppStore } from "./stores/appStore";
import { Sources } from "./pages/Sources";
import "./App.css";

interface AppInfo {
  version: string;
  db_status: string;
}

function App() {
  const { appReady, appVersion, dbStatus, setAppReady, setAppVersion, setDbStatus } = useAppStore();

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

    // Call immediately since the backend `setup` might have emitted the event before React loaded.
    fetchInfo();

    const unlisten = listen("app://ready", fetchInfo);

    return () => {
      unlisten.then(f => f());
    };
  }, [setAppReady, setAppVersion, setDbStatus]);

  return (
    <main className="container" style={{ padding: '2rem', textAlign: 'left', maxWidth: '800px', margin: '0 auto' }}>
      <h1>The Last Place You Look</h1>
      
      <div style={{ marginTop: '2rem', padding: '1rem', border: '1px solid #ccc', borderRadius: '8px' }}>
        <h2>System Status</h2>
        <p><strong>App Ready:</strong> {appReady ? "✅ Yes" : "⏳ Waiting..."}</p>
        <p><strong>Version:</strong> {appVersion || "Loading..."}</p>
        <p><strong>Database:</strong> {dbStatus === "ok" ? "✅ Connected" : (dbStatus || "Loading...")}</p>
      </div>

      <Sources />
    </main>
  );
}

export default App;
