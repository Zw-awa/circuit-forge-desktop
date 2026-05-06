import React from 'react';
import ReactDOM from 'react-dom/client';
import DesktopOverlay from './DesktopOverlay';
import FloatingToolbar from './FloatingToolbar';
import { windowReady } from './ipc/windowIpc';
import './styles/overlay.css';
import './styles/toolbar.css';

function isToolbarWindow(): boolean {
  const params = new URLSearchParams(window.location.search);
  return params.get('w') === 'toolbar';
}

const label: 'overlay' | 'toolbar' = isToolbarWindow() ? 'toolbar' : 'overlay';

const root = ReactDOM.createRoot(document.getElementById('root')!);
root.render(
  <React.StrictMode>
    {label === 'toolbar' ? <FloatingToolbar /> : <DesktopOverlay />}
  </React.StrictMode>,
);

// Notify Rust that this window has finished painting its first frame. Rust
// waits for BOTH windows to report, then shows them in a single operation so
// the user never sees a default white WebView2 frame.
requestAnimationFrame(() => {
  requestAnimationFrame(() => {
    windowReady(label).catch((e) => console.error('window_ready failed:', e));
  });
});


