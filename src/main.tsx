import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";

class ErrorBoundary extends React.Component<{children: React.ReactNode}, {error: Error | null}> {
  constructor(props: any) { super(props); this.state = { error: null }; }
  static getDerivedStateFromError(error: Error) { return { error }; }
  render() {
    if (this.state.error) {
      return (
        <div style={{ padding: '2rem', fontFamily: 'monospace', background: '#1a0000', color: '#ff6b6b', height: '100vh', overflow: 'auto' }}>
          <h2>React Crash</h2>
          <pre style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-all' }}>{this.state.error.message}</pre>
          <pre style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-all', color: '#ff9999', fontSize: '0.8rem' }}>{this.state.error.stack}</pre>
        </div>
      );
    }
    return this.props.children;
  }
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <ErrorBoundary>
    <App />
  </ErrorBoundary>
);
