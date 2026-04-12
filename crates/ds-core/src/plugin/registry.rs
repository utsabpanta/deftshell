use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpmSearchResult {
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub keywords: Vec<String>,
    pub author: Option<NpmAuthor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpmAuthor {
    pub name: Option<String>,
}

pub struct PluginRegistry {
    client: reqwest::Client,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Search npm for DeftShell plugins
    pub async fn search(&self, query: &str) -> Result<Vec<NpmSearchResult>> {
        let url = format!(
            "https://registry.npmjs.org/-/v1/search?text=keywords:deftshell-plugin+{}&size=20",
            query
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .with_context(|| "Failed to search npm registry")?;

        if !response.status().is_success() {
            return Ok(Vec::new());
        }

        let body: serde_json::Value = response.json().await?;
        let results = body["objects"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|obj| {
                        let pkg = &obj["package"];
                        Some(NpmSearchResult {
                            name: pkg["name"].as_str()?.to_string(),
                            description: pkg["description"].as_str().map(String::from),
                            version: pkg["version"].as_str()?.to_string(),
                            keywords: pkg["keywords"]
                                .as_array()
                                .map(|k| {
                                    k.iter()
                                        .filter_map(|v| v.as_str().map(String::from))
                                        .collect()
                                })
                                .unwrap_or_default(),
                            author: pkg["author"]["name"].as_str().map(|n| NpmAuthor {
                                name: Some(n.to_string()),
                            }),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(results)
    }

    /// Scaffold a new plugin project
    pub fn scaffold(name: &str, dest: &std::path::Path) -> Result<()> {
        std::fs::create_dir_all(dest)?;

        // package.json
        let package_json = serde_json::json!({
            "name": name,
            "version": "0.1.0",
            "description": format!("DeftShell plugin: {}", name),
            "main": "index.js",
            "keywords": ["deftshell-plugin"],
            "deftshell": {
                "type": "command"
            },
            "author": "",
            "license": "MIT"
        });
        std::fs::write(
            dest.join("package.json"),
            serde_json::to_string_pretty(&package_json)?,
        )?;

        // index.js
        std::fs::write(
            dest.join("index.js"),
            r#"// DeftShell Plugin: {{name}}
// See https://deftshell.dev/docs/plugins for documentation

module.exports = {
  name: '{{name}}',
  version: '0.1.0',
  type: 'command',

  async onActivate(context) {
    console.log('Plugin activated');
  },

  async onDeactivate() {
    console.log('Plugin deactivated');
  },

  commands: [
    {
      name: 'hello',
      description: 'Say hello from the plugin',
      async handler(args, context) {
        console.log('Hello from {{name}}!');
      }
    }
  ]
};
"#
            .replace("{{name}}", name),
        )?;

        // README.md
        std::fs::write(
            dest.join("README.md"),
            format!(
                "# {}\n\nA DeftShell plugin.\n\n## Installation\n\n```bash\nds plugin install {}\n```\n",
                name, name
            ),
        )?;

        Ok(())
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
