<p align="center">
  <h1 align="center">CircuitForge Desktop</h1>
  <p align="center">Your desktop is the sandbox.</p>
  <p align="center">
    English | <a href="./README_CN.md">中文</a>
  </p>
  <p align="center">
    <a href="https://github.com/Zw-awa/circuit-forge-desktop/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-Apache%202.0-blue.svg" alt="License"></a>
    <img src="https://img.shields.io/badge/platform-Windows%2010%2F11-0078D4?logo=windows" alt="Platform">
    <img src="https://img.shields.io/badge/tauri-v2-FFC131?logo=tauri&logoColor=white" alt="Tauri v2">
    <img src="https://img.shields.io/badge/react-18-61DAFB?logo=react&logoColor=white" alt="React 18">
    <img src="https://img.shields.io/badge/typescript-5-3178C6?logo=typescript&logoColor=white" alt="TypeScript">
    <img src="https://img.shields.io/badge/rust-stable-000000?logo=rust" alt="Rust">
    <img src="https://img.shields.io/badge/status-early%20WIP-orange" alt="Status">
  </p>
</p>

---

**CircuitForge Desktop** is a transparent overlay that floats above the
Windows desktop. Instead of drawing circuits inside a conventional app
window, you draw them directly on top of your wallpaper and icons. A small
floating toolbar — hidden in the system tray when idle — is the only
visible chrome.

> Part of the [CircuitForge](https://github.com/Zw-awa/circuit-forge)
> family. The original project is a conventional desktop app; this one
> re-imagines the same simulation engine as a desktop-overlay experience.

<!-- TODO: add screenshot / demo GIF -->

## Why

Traditional circuit editors live in an app window. That window has to be
maximized, docked, or juggled with the rest of your work. CircuitForge
Desktop takes the opposite approach: the sandbox is *everywhere* and
fades to transparent when you are not actively editing, so your desktop
keeps working normally.

## Status

Early scaffolding. The overlay, toolbar, tray, and Canvas 2D rendering of
seven ANSI gate symbols (AND, OR, NOT, NAND, NOR, XOR, XNOR) are working.
Wiring, simulation wiring-up to the Rust backend, persistence, and most
side panels are not yet implemented.

See [CHANGELOG.md](CHANGELOG.md) for the detailed list.

## Tech stack

| Layer       | Tech                                               |
|-------------|----------------------------------------------------|
| Shell       | [Tauri v2](https://tauri.app) (Windows/WebView2)   |
| Frontend    | React 18, TypeScript, Vite, Zustand                |
| Rendering   | Canvas 2D with pre-rendered SVG halos              |
| Backend     | Rust (simulation engine, scripting, file I/O)      |
| Scripting   | [mlua](https://github.com/mlua-rs/mlua) (Lua 5.4)  |
| Packaging   | [bun](https://bun.sh) for install / scripts        |

## Running

Requires:
- **Windows 10/11** (transparent + always-on-top + click-through is a
  Windows-specific code path right now)
- **Rust 1.75+** with the MSVC toolchain
- **[bun](https://bun.sh)**
- Tauri v2 [system prerequisites](https://tauri.app/start/prerequisites/)

```powershell
# Clone
git clone https://github.com/Zw-awa/circuit-forge-desktop.git
cd circuit-forge-desktop

# Install frontend deps
bun install

# Run in dev mode (must be Windows PowerShell, not WSL — the GUI will not
# render under WSLg for a transparent always-on-top window)
bun run tauri dev
```

## Using the overlay

On first launch you'll see the desktop as usual plus a small floating
toolbar. The app also lives in the system tray.

| Action                       | How                                           |
|------------------------------|-----------------------------------------------|
| Enter edit mode              | Click 🖱️ on the toolbar (becomes ✏️)          |
| Exit edit mode               | `Esc`, or right-click menu → Exit, or tray    |
| Pick a gate                  | Right-click the canvas in edit mode           |
| Place the picked gate        | Left-click (one-shot; cancels afterwards)     |
| Cancel current placement     | Right-click while placing, or `Esc`           |
| Pan / zoom                   | (todo)                                        |
| Move the toolbar             | Drag the ⋮⋮ handle; pin it with 📍           |
| Hide / show toolbar          | Left-click the tray icon                      |
| Quit                         | Toolbar ✕, or tray menu → Quit                |

If anything ever feels "stuck" during edit mode (taskbar or toolbar not
responding to clicks), press `Esc` — overlay drops to click-through and
everything returns to normal.

## Project layout

```
circuit-forge-desktop/
├── src/
│   ├── DesktopOverlay.tsx     Fullscreen transparent canvas + context menu
│   ├── FloatingToolbar.tsx    Small AOT window with controls
│   ├── render/                Canvas 2D renderer (Camera, Renderer, GateImages)
│   ├── stores/                Zustand stores (overlay + circuit)
│   ├── ipc/                   Tauri invoke wrappers
│   ├── types/                 Shared TypeScript types
│   └── assets/gates/          ANSI gate SVGs
└── src-tauri/
    └── src/
        ├── circuit/           Graph, pins, wires, components
        ├── simulation/        Event-driven + tick-driven engines
        ├── scripting/         Lua sandbox (mlua)
        ├── commands/          Tauri #[command] handlers
        │   └── window_cmds.rs Overlay/toolbar window control
        └── lib.rs             Tauri setup, tray, two windows
```

## License

[Apache 2.0](LICENSE). See [NOTICE](NOTICE) for third-party attributions.

Logic gate SVGs are from [Creazilla](https://creazilla.com/) under their
personal-and-commercial-use terms. AND and NOR are derivative works
(modified from NAND and OR respectively).
