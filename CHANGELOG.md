# Changelog

All notable changes to CircuitForge Desktop will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/) and
this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- Initial desktop-overlay scaffolding: fullscreen transparent `overlay`
  window covering the Windows desktop, paired with a small floating
  `toolbar` window hosted in the system tray.
- System tray icon with left-click show/hide, right-click menu (show,
  hide, exit edit mode, quit).
- Toolbar controls:
  - Position lock (📍 / 📌) to prevent accidental drag.
  - Edit-mode toggle (🖱️ / ✏️) that flips overlay click-through.
  - App exit button.
- Three independent escape paths from edit mode: `Esc`, in-canvas right-
  click menu, and tray menu entry.
- Canvas 2D renderer with:
  - Camera (pan + zoom + DPR handling).
  - Dot grid.
  - Pre-rendered white halos so gates read on any desktop background.
  - Seven ANSI logic gate symbols: AND, OR, NOT, NAND, NOR, XOR, XNOR.
- One-shot placement flow: right-click → pick gate → left-click to drop.
  Right-click during placement cancels without opening the menu.
- Silent startup: both windows load invisible and `show()` only after the
  frontend reports its first paint, eliminating the default-background
  flash.
- Rust command `window_ready` that atomically sizes the overlay to the
  primary monitor and reveals both windows once the React app has
  committed its first frame.

### Known limitations
- Gate placement is frontend-only; Rust simulation backend is present but
  not yet wired to the desktop UI.
- No wiring, selection, or deletion tools yet.
- No persistence (placed gates are lost on app exit).
- Windows-only; macOS/Linux behavior of transparent + always-on-top +
  click-through windows has not been validated.
