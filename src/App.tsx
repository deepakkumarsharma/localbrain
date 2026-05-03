import { useEffect, useState } from 'react';
import type { CSSProperties, PointerEvent } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { MainPanel } from './components/MainPanel';
import { RightPanel } from './components/RightPanel';
import { Sidebar } from './components/Sidebar';
import { useAppStore } from './store/useAppStore';
import { initFileWatcher } from './lib/fileWatcher';

const LARGE_SCREEN_SIDE_PANEL_MIN_WIDTH = 400;
const LARGE_SCREEN_SIDE_PANEL_MAX_WIDTH = 600;
const SMALL_SCREEN_SIDE_PANEL_MIN_WIDTH = 260;
const SMALL_SCREEN_SIDE_PANEL_MAX_WIDTH = 380;
const LEFT_PANEL_DEFAULT_WIDTH = 320;
const RIGHT_PANEL_DEFAULT_WIDTH = 360;
const LARGE_SCREEN_BREAKPOINT = 1440;

export default function App() {
  const { setAppVersion, theme, toggleTheme } = useAppStore();
  const [leftPanelWidth, setLeftPanelWidth] = useState(LEFT_PANEL_DEFAULT_WIDTH);
  const [rightPanelWidth, setRightPanelWidth] = useState(RIGHT_PANEL_DEFAULT_WIDTH);

  useEffect(() => {
    void invoke<string>('get_app_version')
      .then(setAppVersion)
      .catch(() => setAppVersion('unknown'));
  }, [setAppVersion]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    void initFileWatcher('.')
      .then((ul) => {
        unlisten = ul;
      })
      .catch(console.error);

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);

  useEffect(() => {
    function handleWindowResize() {
      const bounds = getSidePanelBounds();
      setLeftPanelWidth((width) => clamp(width, bounds.min, bounds.max));
      setRightPanelWidth((width) => clamp(width, bounds.min, bounds.max));
    }

    handleWindowResize();
    window.addEventListener('resize', handleWindowResize);

    return () => window.removeEventListener('resize', handleWindowResize);
  }, []);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.repeat) {
        return;
      }

      if ((event.metaKey || event.ctrlKey) && event.shiftKey && event.key.toLowerCase() === 't') {
        event.preventDefault();
        toggleTheme();
      }
    }

    window.addEventListener('keydown', handleKeyDown);

    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [toggleTheme]);

  function handleLeftPanelResizeStart(event: PointerEvent<HTMLDivElement>) {
    startPanelResize(event, (clientX) => clientX, setLeftPanelWidth);
  }

  function handleRightPanelResizeStart(event: PointerEvent<HTMLDivElement>) {
    startPanelResize(event, (clientX) => window.innerWidth - clientX, setRightPanelWidth);
  }

  function startPanelResize(
    event: PointerEvent<HTMLDivElement>,
    getNextWidth: (clientX: number) => number,
    setPanelWidth: (width: number) => void,
  ) {
    event.preventDefault();

    const previousCursor = document.body.style.cursor;
    const previousUserSelect = document.body.style.userSelect;
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';

    function handlePointerMove(moveEvent: globalThis.PointerEvent) {
      const bounds = getSidePanelBounds();
      const nextWidth = getNextWidth(moveEvent.clientX);
      setPanelWidth(clamp(nextWidth, bounds.min, bounds.max));
    }

    function handlePointerUp() {
      document.body.style.cursor = previousCursor;
      document.body.style.userSelect = previousUserSelect;
      window.removeEventListener('pointermove', handlePointerMove);
      window.removeEventListener('pointerup', handlePointerUp);
    }

    window.addEventListener('pointermove', handlePointerMove);
    window.addEventListener('pointerup', handlePointerUp);
  }

  const layoutStyle = {
    '--left-panel-width': `${leftPanelWidth}px`,
    '--right-panel-width': `${rightPanelWidth}px`,
  } as CSSProperties;

  return (
    <div
      className="grid h-screen min-w-[1024px] grid-cols-[var(--left-panel-width)_6px_minmax(0,1fr)_6px_var(--right-panel-width)] overflow-hidden bg-app-background text-app-text"
      style={layoutStyle}
    >
      <Sidebar />
      <div
        className="group flex cursor-col-resize items-stretch bg-app-background"
        role="separator"
        aria-label="Resize sidebar"
        aria-orientation="vertical"
        onPointerDown={handleLeftPanelResizeStart}
      >
        <div className="mx-auto w-px bg-app-border transition-colors group-hover:bg-app-accent" />
      </div>
      <MainPanel />
      <div
        className="group flex cursor-col-resize items-stretch bg-app-background"
        role="separator"
        aria-label="Resize details panel"
        aria-orientation="vertical"
        onPointerDown={handleRightPanelResizeStart}
      >
        <div className="mx-auto w-px bg-app-border transition-colors group-hover:bg-app-accent" />
      </div>
      <RightPanel />
    </div>
  );
}

function getSidePanelBounds() {
  if (window.innerWidth >= LARGE_SCREEN_BREAKPOINT) {
    return {
      min: LARGE_SCREEN_SIDE_PANEL_MIN_WIDTH,
      max: LARGE_SCREEN_SIDE_PANEL_MAX_WIDTH,
    };
  }

  return {
    min: SMALL_SCREEN_SIDE_PANEL_MIN_WIDTH,
    max: SMALL_SCREEN_SIDE_PANEL_MAX_WIDTH,
  };
}

function clamp(value: number, min: number, max: number) {
  return Math.min(max, Math.max(min, value));
}
