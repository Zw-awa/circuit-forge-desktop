# CircuitForge Desktop

> 把 Windows 桌面本身变成一块逻辑电路画板。
> Turn your Windows desktop into a logic circuit sandbox.

**English** | [中文](#中文)

---

## English

CircuitForge Desktop is a transparent overlay that floats above the Windows
desktop. Instead of drawing circuits inside a conventional app window, you
draw them directly on top of your wallpaper and icons. A small floating
toolbar — hidden in the system tray when idle — is the only visible chrome.

### Why

Traditional circuit editors live in an app window. That window has to be
maximized, docked, or juggled with the rest of your work. CircuitForge
Desktop takes the opposite approach: the sandbox is *everywhere* and
fades to transparent when you are not actively editing, so your desktop
keeps working normally.

### Status

Early scaffolding. The overlay, toolbar, tray, and Canvas 2D rendering of
seven ANSI gate symbols (AND, OR, NOT, NAND, NOR, XOR, XNOR) are working.
Wiring, simulation wiring-up to the Rust backend, persistence, and most
side panels are not yet implemented.

See [CHANGELOG.md](CHANGELOG.md) for the detailed list.

### Tech stack

| Layer       | Tech                                               |
|-------------|----------------------------------------------------|
| Shell       | [Tauri v2](https://tauri.app) (Windows/WebView2)   |
| Frontend    | React 18, TypeScript, Vite, Zustand                |
| Rendering   | Canvas 2D with pre-rendered SVG halos              |
| Backend     | Rust (simulation engine, scripting, file I/O)      |
| Scripting   | [mlua](https://github.com/mlua-rs/mlua) (Lua 5.4)  |
| Packaging   | [bun](https://bun.sh) for install / scripts        |

### Running

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

### Using the overlay

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

### Project layout

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

### License

[Apache 2.0](LICENSE). See [NOTICE](NOTICE) for third-party attributions.

Logic gate SVGs are from [Creazilla](https://creazilla.com/) under their
personal-and-commercial-use terms. AND and NOR are derivative works
(modified from NAND and OR respectively).

---

## 中文

CircuitForge Desktop 是一个**透明的桌面叠加层**。它不像普通软件那样把电路
编辑器关在应用窗口里，而是让你直接在桌面壁纸和图标之上画逻辑电路。空闲时
界面完全透明，只有一条可藏到系统托盘的小浮动工具栏。

### 为什么做这个

常规的电路编辑器要你在一个应用窗口里工作 —— 窗口要么占着屏幕，要么和别的
事情抢空间。CircuitForge Desktop 反其道而行：画板**始终就在那里**，不编辑
时完全看不见，不打扰你用电脑。

### 当前状态

早期阶段。透明覆盖层、浮动工具栏、系统托盘、基于 Canvas 2D 的七种 ANSI 门
电路符号（AND、OR、NOT、NAND、NOR、XOR、XNOR）已经跑通。连线、对接 Rust
仿真后端、持久化、辅助面板还没做。

详细变更见 [CHANGELOG.md](CHANGELOG.md)。

### 技术栈

| 层         | 技术                                               |
|-----------|----------------------------------------------------|
| 桌面框架   | [Tauri v2](https://tauri.app)（Windows/WebView2）   |
| 前端       | React 18、TypeScript、Vite、Zustand                 |
| 渲染       | Canvas 2D + 预渲染 SVG 白色描边                     |
| 后端       | Rust（仿真引擎、脚本、文件 IO）                     |
| 脚本       | [mlua](https://github.com/mlua-rs/mlua)（Lua 5.4）  |
| 包管理     | [bun](https://bun.sh)                               |

### 运行

环境要求：
- **Windows 10/11**（透明 + 始终置顶 + 鼠标穿透目前是 Windows 专属实现）
- **Rust 1.75+**（MSVC 工具链）
- **[bun](https://bun.sh)**
- Tauri v2 [系统依赖](https://tauri.app/start/prerequisites/)

```powershell
# 克隆
git clone https://github.com/Zw-awa/circuit-forge-desktop.git
cd circuit-forge-desktop

# 安装前端依赖
bun install

# 开发模式（必须用 Windows PowerShell，不要用 WSL —— WSLg 无法正确显示
# 透明置顶窗口）
bun run tauri dev
```

### 使用方法

启动后你会看到平常的桌面加一条小浮动工具栏。应用也在系统托盘里。

| 操作              | 方式                                                |
|-------------------|-----------------------------------------------------|
| 进入编辑模式      | 点工具栏的 🖱️（变 ✏️）                             |
| 退出编辑模式      | `Esc`、右键菜单"退出编辑模式"、或托盘菜单           |
| 选择元件          | 编辑模式下右键画布                                  |
| 放置选中的元件    | 左键（一次一个，放下后自动退出放置）                |
| 取消当前放置      | 放置中右键、或 `Esc`                                |
| 平移/缩放         | （待实现）                                          |
| 移动工具栏        | 拖 ⋮⋮ 手柄；📍 可以锁定位置                         |
| 隐藏/显示工具栏   | 左键托盘图标                                        |
| 退出              | 工具栏 ✕、或托盘菜单"退出"                          |

如果编辑模式下出现任何"卡住"的感觉（任务栏或工具栏点不动），按 `Esc` ——
覆盖层会立即切回穿透模式，一切恢复正常。

### 项目结构

见上面英文版的 Project layout。

### 许可证

[Apache 2.0](LICENSE)。第三方素材来源见 [NOTICE](NOTICE)。

门电路 SVG 素材来自 [Creazilla](https://creazilla.com/)，遵循其允许个人
和商业使用的条款。AND 和 NOR 为改编作品（分别在 NAND 和 OR 的基础上修改）。
