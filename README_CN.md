<p align="center">
  <h1 align="center">CircuitForge Desktop</h1>
  <p align="center">桌面即画板</p>
  <p align="center">
    <a href="./README.md">English</a> | 中文
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

**CircuitForge Desktop** 是一个**透明的桌面叠加层**。它不像普通软件那样把
电路编辑器关在应用窗口里，而是让你直接在桌面壁纸和图标之上画逻辑电路。
空闲时界面完全透明，只有一条可藏到系统托盘的小浮动工具栏。

> 属于 [CircuitForge](https://github.com/Zw-awa/circuit-forge) 家族。原项目
> 是传统桌面应用形态；本项目沿用同一套仿真引擎，重新设计为桌面叠加体验。

<!-- TODO: 添加应用截图 / 演示动图 -->

## 为什么做这个

常规的电路编辑器要你在一个应用窗口里工作 —— 窗口要么占着屏幕，要么和别的
事情抢空间。CircuitForge Desktop 反其道而行：画板**始终就在那里**，不编辑
时完全看不见，不打扰你用电脑。

## 当前状态

早期阶段。透明覆盖层、浮动工具栏、系统托盘、基于 Canvas 2D 的七种 ANSI 门
电路符号（AND、OR、NOT、NAND、NOR、XOR、XNOR）已经跑通。连线、对接 Rust
仿真后端、持久化、辅助面板还没做。

详细变更见 [CHANGELOG.md](CHANGELOG.md)。

## 技术栈

| 层         | 技术                                               |
|-----------|----------------------------------------------------|
| 桌面框架   | [Tauri v2](https://tauri.app)（Windows/WebView2）   |
| 前端       | React 18、TypeScript、Vite、Zustand                 |
| 渲染       | Canvas 2D + 预渲染 SVG 白色描边                     |
| 后端       | Rust（仿真引擎、脚本、文件 IO）                     |
| 脚本       | [mlua](https://github.com/mlua-rs/mlua)（Lua 5.4）  |
| 包管理     | [bun](https://bun.sh)                               |

## 运行

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

## 使用方法

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

## 项目结构

```
circuit-forge-desktop/
├── src/
│   ├── DesktopOverlay.tsx     全屏透明画布 + 上下文菜单
│   ├── FloatingToolbar.tsx    浮动工具栏（独立 AOT 窗口）
│   ├── render/                Canvas 2D 渲染层（Camera、Renderer、GateImages）
│   ├── stores/                Zustand 状态（overlay + circuit）
│   ├── ipc/                   Tauri invoke 包装
│   ├── types/                 共享的 TypeScript 类型
│   └── assets/gates/          ANSI 门电路 SVG
└── src-tauri/
    └── src/
        ├── circuit/           电路图（节点、引脚、连线）
        ├── simulation/        事件驱动 + 时钟帧驱动仿真引擎
        ├── scripting/         Lua 沙盒（mlua）
        ├── commands/          Tauri #[command] 处理器
        │   └── window_cmds.rs 覆盖层 / 工具栏窗口控制
        └── lib.rs             Tauri 启动、托盘、双窗口
```

## 许可证

[Apache 2.0](LICENSE)。第三方素材来源见 [NOTICE](NOTICE)。

门电路 SVG 素材来自 [Creazilla](https://creazilla.com/)，遵循其允许个人和
商业使用的条款。AND 和 NOR 为改编作品（分别在 NAND 和 OR 的基础上修改）。
