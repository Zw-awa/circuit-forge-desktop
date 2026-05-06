use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Serialize, Deserialize)]
pub struct KeyBinding {
    pub key: String,
    pub action: String,
    pub description: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct KeyBindingConfig {
    pub bindings: Vec<KeyBinding>,
}

pub fn default_bindings() -> KeyBindingConfig {
    KeyBindingConfig {
        bindings: vec![
            // Tool switching
            KeyBinding {
                key: "1".into(),
                action: "tool.select".into(),
                description: "选择工具".into(),
            },
            KeyBinding {
                key: "2".into(),
                action: "tool.place".into(),
                description: "放置工具".into(),
            },
            KeyBinding {
                key: "3".into(),
                action: "tool.wire".into(),
                description: "连线工具".into(),
            },
            KeyBinding {
                key: "4".into(),
                action: "tool.delete".into(),
                description: "删除工具".into(),
            },
            // Canvas
            KeyBinding {
                key: "Escape".into(),
                action: "canvas.escape".into(),
                description: "取消/返回选择".into(),
            },
            KeyBinding {
                key: "Space".into(),
                action: "canvas.pan".into(),
                description: "平移画布(按住)".into(),
            },
            KeyBinding {
                key: "F".into(),
                action: "canvas.freeMode".into(),
                description: "自由模式".into(),
            },
            KeyBinding {
                key: "G".into(),
                action: "canvas.gridMode".into(),
                description: "网格模式".into(),
            },
            // Editing
            KeyBinding {
                key: "Ctrl+Z".into(),
                action: "edit.undo".into(),
                description: "撤销".into(),
            },
            KeyBinding {
                key: "Ctrl+Shift+Z".into(),
                action: "edit.redo".into(),
                description: "重做".into(),
            },
            KeyBinding {
                key: "Ctrl+Y".into(),
                action: "edit.redo_alt".into(),
                description: "重做(备用)".into(),
            },
            KeyBinding {
                key: "Delete".into(),
                action: "edit.delete".into(),
                description: "删除选中".into(),
            },
            KeyBinding {
                key: "Backspace".into(),
                action: "edit.delete_alt".into(),
                description: "删除选中(备用)".into(),
            },
            // File
            KeyBinding {
                key: "Ctrl+S".into(),
                action: "file.save".into(),
                description: "保存".into(),
            },
            KeyBinding {
                key: "Ctrl+O".into(),
                action: "file.open".into(),
                description: "打开".into(),
            },
            // Simulation
            KeyBinding {
                key: "V".into(),
                action: "sim.signalDisplay".into(),
                description: "信号显示模式".into(),
            },
            // Snapshot
            KeyBinding {
                key: "Ctrl+Shift+S".into(),
                action: "snapshot.create".into(),
                description: "创建快照".into(),
            },
            // Theme
            KeyBinding {
                key: "Ctrl+Shift+T".into(),
                action: "theme.toggle".into(),
                description: "切换主题".into(),
            },
        ],
    }
}

pub fn save_bindings(config: &KeyBindingConfig) -> Result<(), String> {
    let path = get_bindings_path()?;
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

pub fn load_bindings() -> Result<KeyBindingConfig, String> {
    let path = get_bindings_path()?;
    if !path.exists() {
        let default = default_bindings();
        save_bindings(&default)?;
        return Ok(default);
    }
    let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut config: KeyBindingConfig =
        serde_json::from_str(&json).map_err(|_| String::from("invalid binding config"))?;

    // Merge: if any default action is missing from loaded config, add it
    let defaults = default_bindings();
    let loaded_actions: std::collections::HashSet<String> =
        config.bindings.iter().map(|b| b.action.clone()).collect();
    for default_binding in defaults.bindings {
        if !loaded_actions.contains(&default_binding.action) {
            config.bindings.push(default_binding);
        }
    }

    Ok(config)
}

fn get_bindings_path() -> Result<PathBuf, String> {
    let dir = if cfg!(target_os = "windows") {
        std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir())
            .join("circuit-forge")
    } else if cfg!(target_os = "macos") {
        dirs_fallback("HOME", "Library/Application Support/circuit-forge")
    } else {
        dirs_fallback("XDG_CONFIG_HOME", ".config/circuit-forge")
    };
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("keybindings.json"))
}

fn dirs_fallback(env_var: &str, relative: &str) -> PathBuf {
    std::env::var(env_var)
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| String::from("."));
            PathBuf::from(home).join(relative)
        })
}
