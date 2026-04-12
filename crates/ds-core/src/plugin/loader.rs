use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub plugin_type: String,
    pub entry_point: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub path: PathBuf,
    pub enabled: bool,
}

pub struct PluginLoader {
    plugins_dir: PathBuf,
}

impl PluginLoader {
    pub fn new(plugins_dir: PathBuf) -> Self {
        Self { plugins_dir }
    }

    /// Load all plugins from the plugins directory
    pub fn load_all(&self) -> Result<Vec<LoadedPlugin>> {
        let mut plugins = Vec::new();

        if !self.plugins_dir.exists() {
            return Ok(plugins);
        }

        for entry in std::fs::read_dir(&self.plugins_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                match self.load_plugin(&path) {
                    Ok(plugin) => plugins.push(plugin),
                    Err(e) => {
                        tracing::warn!("Failed to load plugin at {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(plugins)
    }

    /// Load a single plugin from a directory
    pub fn load_plugin(&self, path: &Path) -> Result<LoadedPlugin> {
        // Look for manifest in package.json or plugin.toml
        let manifest = if path.join("package.json").exists() {
            self.load_npm_manifest(path)?
        } else if path.join("plugin.toml").exists() {
            self.load_toml_manifest(path)?
        } else {
            anyhow::bail!("No plugin manifest found in {}", path.display());
        };

        // Check for disabled marker
        let enabled = !path.join(".disabled").exists();

        Ok(LoadedPlugin {
            manifest,
            path: path.to_path_buf(),
            enabled,
        })
    }

    /// Validate that a plugin name is safe for use in filesystem paths.
    fn validate_plugin_name(name: &str) -> Result<()> {
        if name.is_empty() {
            anyhow::bail!("Plugin name cannot be empty");
        }
        if name.contains("..") || name.contains('/') || name.contains('\\') || name.starts_with('.')
        {
            anyhow::bail!(
                "Invalid plugin name '{}': must not contain path separators or '..'",
                name
            );
        }
        Ok(())
    }

    /// Install a plugin from npm
    pub fn install_from_npm(&self, name: &str) -> Result<LoadedPlugin> {
        Self::validate_plugin_name(name)?;
        let plugin_dir = self.plugins_dir.join(name);
        std::fs::create_dir_all(&plugin_dir)?;

        // Run npm install in the plugin directory
        let output = std::process::Command::new("npm")
            .args(["install", "--prefix", &plugin_dir.to_string_lossy(), name])
            .output()
            .with_context(|| format!("Failed to install plugin: {}", name))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("npm install failed: {}", stderr);
        }

        // The actual package will be in node_modules
        let package_path = plugin_dir.join("node_modules").join(name);
        self.load_plugin(&package_path)
    }

    /// Install a plugin from a local directory
    pub fn install_from_local(&self, source: &Path) -> Result<LoadedPlugin> {
        let manifest = if source.join("package.json").exists() {
            self.load_npm_manifest(source)?
        } else if source.join("plugin.toml").exists() {
            self.load_toml_manifest(source)?
        } else {
            anyhow::bail!("No plugin manifest found in {}", source.display());
        };

        let dest = self.plugins_dir.join(&manifest.name);
        if dest.exists() {
            std::fs::remove_dir_all(&dest)?;
        }

        // Copy plugin files
        Self::copy_dir_recursive(source, &dest)?;

        self.load_plugin(&dest)
    }

    /// Remove a plugin
    pub fn remove(&self, name: &str) -> Result<()> {
        Self::validate_plugin_name(name)?;
        let plugin_dir = self.plugins_dir.join(name);
        if plugin_dir.exists() {
            std::fs::remove_dir_all(&plugin_dir)?;
        }
        Ok(())
    }

    /// Enable a plugin
    pub fn enable(&self, name: &str) -> Result<()> {
        let disabled_marker = self.plugins_dir.join(name).join(".disabled");
        if disabled_marker.exists() {
            std::fs::remove_file(disabled_marker)?;
        }
        Ok(())
    }

    /// Disable a plugin
    pub fn disable(&self, name: &str) -> Result<()> {
        let disabled_marker = self.plugins_dir.join(name).join(".disabled");
        std::fs::write(disabled_marker, "")?;
        Ok(())
    }

    fn load_npm_manifest(&self, path: &Path) -> Result<PluginManifest> {
        let pkg_json_path = path.join("package.json");
        let contents = std::fs::read_to_string(&pkg_json_path)?;
        let pkg: serde_json::Value = serde_json::from_str(&contents)?;

        Ok(PluginManifest {
            name: pkg["name"].as_str().unwrap_or("unknown").to_string(),
            version: pkg["version"].as_str().unwrap_or("0.0.0").to_string(),
            description: pkg["description"].as_str().unwrap_or("").to_string(),
            author: pkg["author"]
                .as_str()
                .or_else(|| pkg["author"]["name"].as_str())
                .unwrap_or("")
                .to_string(),
            plugin_type: pkg["deftshell"]["type"]
                .as_str()
                .unwrap_or("command")
                .to_string(),
            entry_point: pkg["main"].as_str().map(String::from),
            homepage: pkg["homepage"].as_str().map(String::from),
            repository: pkg["repository"]["url"].as_str().map(String::from),
            keywords: pkg["keywords"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
        })
    }

    fn load_toml_manifest(&self, path: &Path) -> Result<PluginManifest> {
        let manifest_path = path.join("plugin.toml");
        let contents = std::fs::read_to_string(&manifest_path)?;
        let manifest: PluginManifest = toml::from_str(&contents)?;
        Ok(manifest)
    }

    fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            if src_path.is_dir() {
                Self::copy_dir_recursive(&src_path, &dst_path)?;
            } else {
                std::fs::copy(&src_path, &dst_path)?;
            }
        }
        Ok(())
    }
}
