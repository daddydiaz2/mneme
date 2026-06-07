use std::path::{Path, PathBuf};

use serde_json::json;
use tracing::{debug, warn};

use crate::error::{MnemeError, Result};

use super::manifest::{PluginManifest, PluginTool};

// ── Internal representation of a loaded plugin ──────────────────────────────

#[derive(Debug)]
struct LoadedPlugin {
    manifest: PluginManifest,
    /// Raw WASM bytes, kept for re-instantiation per call.
    #[allow(dead_code)]
    wasm_bytes: Vec<u8>,
}

// ── PluginManager ────────────────────────────────────────────────────────────

/// Manages WASM plugins loaded from disk.
///
/// Plugins are discovered from `~/.config/mneme/plugins/*.wasm` at startup.
/// Each plugin is sandboxed and communicates via JSON over the extism ABI.
#[derive(Debug)]
pub struct PluginManager {
    plugins: Vec<LoadedPlugin>,
}

impl PluginManager {
    /// Returns an empty manager (no plugins loaded).
    pub fn empty() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// Discover and load plugins from the default directory:
    /// `~/.config/mneme/plugins/*.wasm`
    pub fn load_from_default_dir() -> Result<Self> {
        let dir = dirs::config_dir()
            .map(|d| d.join("mneme").join("plugins"))
            .ok_or_else(|| MnemeError::Plugin("cannot resolve config directory".into()))?;
        Self::load_from_dir(&dir)
    }

    /// Load plugins from an explicit directory (useful for tests).
    pub fn load_from_dir(dir: &Path) -> Result<Self> {
        if !dir.exists() {
            debug!(path = %dir.display(), "plugin directory does not exist, skipping");
            return Ok(Self::empty());
        }

        let mut plugins = Vec::new();

        let entries = std::fs::read_dir(dir)
            .map_err(|e| MnemeError::Plugin(format!("cannot read plugin dir: {}", e)))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("wasm") {
                continue;
            }

            match Self::load_one(&path) {
                Ok(plugin) => {
                    debug!(name = %plugin.manifest.name, path = %path.display(), "plugin loaded");
                    plugins.push(plugin);
                }
                Err(e) => {
                    warn!(path = %path.display(), error = %e, "failed to load plugin, skipping");
                }
            }
        }

        Ok(Self { plugins })
    }

    /// Load a single .wasm file and read its manifest.
    fn load_one(path: &PathBuf) -> Result<LoadedPlugin> {
        let wasm_bytes = std::fs::read(path)
            .map_err(|e| MnemeError::Plugin(format!("cannot read {}: {}", path.display(), e)))?;

        let manifest = Self::call_manifest(&wasm_bytes)?;

        Ok(LoadedPlugin {
            manifest,
            wasm_bytes,
        })
    }

    /// Invoke the `plugin_manifest` export and parse the result.
    fn call_manifest(wasm_bytes: &[u8]) -> Result<PluginManifest> {
        #[cfg(feature = "plugins")]
        {
            use extism::{Manifest as ExtismManifest, Plugin, Wasm};

            let wasm = Wasm::data(wasm_bytes.to_vec());
            let ext_manifest = ExtismManifest::new([wasm]);
            let mut plugin = Plugin::new(&ext_manifest, [], true)
                .map_err(|e| MnemeError::Plugin(format!("plugin init failed: {}", e)))?;

            let raw: Vec<u8> = plugin
                .call::<&[u8], Vec<u8>>("plugin_manifest", b"")
                .map_err(|e| MnemeError::Plugin(format!("plugin_manifest call failed: {}", e)))?
                .to_vec();

            let manifest: PluginManifest = serde_json::from_slice(&raw)
                .map_err(|e| MnemeError::Plugin(format!("invalid manifest JSON: {}", e)))?;

            Ok(manifest)
        }

        #[cfg(not(feature = "plugins"))]
        {
            let _ = wasm_bytes;
            Err(MnemeError::Plugin(
                "compiled without 'plugins' feature".into(),
            ))
        }
    }

    // ── Public query API ──────────────────────────────────────────────────────

    /// Returns `true` if no plugins are loaded.
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    /// All tools provided by loaded plugins.
    pub fn plugin_tools(&self) -> Vec<PluginTool> {
        self.plugins
            .iter()
            .flat_map(|p| p.manifest.tools.iter().cloned())
            .collect()
    }

    /// Returns `true` if the given tool name belongs to any loaded plugin.
    pub fn owns_tool(&self, tool_name: &str) -> bool {
        self.plugins
            .iter()
            .any(|p| p.manifest.tools.iter().any(|t| t.name == tool_name))
    }

    // ── Dispatch ──────────────────────────────────────────────────────────────

    /// Dispatch a tool call to the plugin that owns it.
    ///
    /// Returns `MnemeError::Plugin` if no plugin owns the tool.
    pub fn call_tool(
        &self,
        tool_name: &str,
        args: serde_json::Value,
        project: &str,
    ) -> Result<serde_json::Value> {
        let plugin = self
            .plugins
            .iter()
            .find(|p| p.manifest.tools.iter().any(|t| t.name == tool_name))
            .ok_or_else(|| MnemeError::Plugin(format!("no plugin owns tool '{}'", tool_name)))?;

        let payload = json!({
            "tool": tool_name,
            "args": args,
            "project": project,
        });

        self.invoke_plugin(plugin, "call_tool", &payload)
    }

    /// Run the `pre_save` hook through all plugins that declare it.
    ///
    /// Plugins are chained: the output of one becomes the input of the next.
    pub fn run_pre_save(&self, memory: serde_json::Value) -> Result<serde_json::Value> {
        self.run_transform_hook("pre_save", memory)
    }

    /// Run the `post_get` hook through all plugins that declare it.
    pub fn run_post_get(&self, memory: serde_json::Value) -> Result<serde_json::Value> {
        self.run_transform_hook("post_get", memory)
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn run_transform_hook(
        &self,
        hook: &str,
        mut memory: serde_json::Value,
    ) -> Result<serde_json::Value> {
        for plugin in &self.plugins {
            if !plugin.manifest.hooks.iter().any(|h| h == hook) {
                continue;
            }
            let payload = json!({ "hook": hook, "memory": memory });
            let result = self.invoke_plugin(plugin, "transform_memory", &payload)?;
            memory = result.get("memory").cloned().unwrap_or(result);
        }
        Ok(memory)
    }

    fn invoke_plugin(
        &self,
        plugin: &LoadedPlugin,
        func: &str,
        payload: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        #[cfg(feature = "plugins")]
        {
            use extism::{Manifest as ExtismManifest, Plugin, Wasm};

            let input = serde_json::to_vec(payload)
                .map_err(|e| MnemeError::Plugin(format!("serialize input: {}", e)))?;

            let wasm = Wasm::data(plugin.wasm_bytes.clone());
            let ext_manifest = ExtismManifest::new([wasm]);
            let mut instance = Plugin::new(&ext_manifest, [], true)
                .map_err(|e| MnemeError::Plugin(format!("plugin init: {}", e)))?;

            let raw: Vec<u8> = instance
                .call::<Vec<u8>, Vec<u8>>(func, input)
                .map_err(|e| {
                    MnemeError::Plugin(format!(
                        "plugin '{}' call '{}' failed: {}",
                        plugin.manifest.name, func, e
                    ))
                })?
                .to_vec();

            serde_json::from_slice(&raw)
                .map_err(|e| MnemeError::Plugin(format!("invalid response JSON: {}", e)))
        }

        #[cfg(not(feature = "plugins"))]
        {
            let _ = (plugin, func, payload);
            Err(MnemeError::Plugin(
                "compiled without 'plugins' feature".into(),
            ))
        }
    }
}
