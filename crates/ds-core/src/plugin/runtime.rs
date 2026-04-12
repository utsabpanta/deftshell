use super::loader::LoadedPlugin;
use anyhow::Result;
use std::collections::HashMap;

/// Plugin runtime manages loaded plugin instances
pub struct PluginRuntime {
    plugins: HashMap<String, LoadedPlugin>,
}

impl PluginRuntime {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    /// Register a loaded plugin
    pub fn register(&mut self, plugin: LoadedPlugin) {
        if plugin.enabled {
            tracing::info!(
                "Registered plugin: {} v{}",
                plugin.manifest.name,
                plugin.manifest.version
            );
            self.plugins.insert(plugin.manifest.name.clone(), plugin);
        }
    }

    /// Get a plugin by name
    pub fn get(&self, name: &str) -> Option<&LoadedPlugin> {
        self.plugins.get(name)
    }

    /// List all registered plugins
    pub fn list(&self) -> Vec<&LoadedPlugin> {
        self.plugins.values().collect()
    }

    /// Execute a plugin command (runs the plugin's Node.js entry point)
    pub fn execute_command(
        &self,
        plugin_name: &str,
        command: &str,
        args: &[String],
    ) -> Result<String> {
        let plugin = self
            .plugins
            .get(plugin_name)
            .ok_or_else(|| anyhow::anyhow!("Plugin not found: {}", plugin_name))?;

        let entry = plugin.manifest.entry_point.as_deref().unwrap_or("index.js");
        let entry_path = plugin.path.join(entry);

        if !entry_path.exists() {
            anyhow::bail!("Plugin entry point not found: {}", entry_path.display());
        }

        // Execute via Node.js
        let output = std::process::Command::new("node")
            .arg(&entry_path)
            .arg(command)
            .args(args)
            .output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Plugin command failed: {}", stderr);
        }
    }

    /// Notify all plugins of a project detection event
    pub fn on_project_detected(&self, context_json: &str) -> Result<()> {
        for plugin in self.plugins.values() {
            if let Some(ref entry) = plugin.manifest.entry_point {
                let entry_path = plugin.path.join(entry);
                if entry_path.exists() {
                    // Fire-and-forget notification
                    let _ = std::process::Command::new("node")
                        .arg(&entry_path)
                        .arg("on-project-detected")
                        .arg(context_json)
                        .spawn();
                }
            }
        }
        Ok(())
    }
}

impl Default for PluginRuntime {
    fn default() -> Self {
        Self::new()
    }
}
