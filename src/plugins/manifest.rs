use serde::{Deserialize, Serialize};

/// Manifest declared by a WASM plugin via its `plugin_manifest` export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Unique plugin identifier.
    pub name: String,
    /// Semver version string.
    pub version: String,
    /// Human-readable description.
    pub description: String,
    /// MCP tools this plugin exposes.
    pub tools: Vec<PluginTool>,
    /// Memory lifecycle hooks: "pre_save", "post_get".
    #[serde(default)]
    pub hooks: Vec<String>,
}

/// A single MCP tool declared by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginTool {
    /// Tool name (must be globally unique, e.g. "myplugin_summarize").
    pub name: String,
    /// Human-readable description shown to MCP clients.
    pub description: String,
    /// JSON Schema for the tool's input parameters.
    pub input_schema: serde_json::Value,
}
