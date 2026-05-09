import React from 'react';
import type { ErrorInfo, ReactNode } from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './styles.css';

class RootErrorBoundary extends React.Component<{ children: ReactNode }, { error: Error | null }> {
  state: { error: Error | null } = { error: null };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error('Localbrain render crash:', error, info.componentStack);
  }

  render() {
    if (this.state.error) {
      return (
        <div className="flex h-screen items-center justify-center bg-app-background p-8 text-app-text">
          <div className="max-w-xl rounded-2xl border border-app-border bg-app-panel p-6 shadow-xl">
            <div className="text-sm font-black uppercase tracking-widest text-app-error">
              Render error
            </div>
            <h1 className="mt-3 text-2xl font-black">Local Brain hit a UI error.</h1>
            <p className="mt-2 text-sm text-app-muted">
              The app stayed open instead of going blank. Restart the window after copying this
              error if needed.
            </p>
            <pre className="mt-4 max-h-56 overflow-auto rounded-xl border border-app-border bg-app-panelSoft p-3 text-xs text-app-text">
              {this.state.error.message}
            </pre>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <RootErrorBoundary>
      <App />
    </RootErrorBoundary>
  </React.StrictMode>,
);
