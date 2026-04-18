use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryIndex {
    pub runbooks: Vec<RegistryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub name: String,
    pub author: String,
    pub description: String,
    pub version: String,
    pub tags: Vec<String>,
    pub source_url: String,
    pub stars: u64,
    pub downloads: u64,
    pub created_at: String,
    pub updated_at: String,
}

pub struct RunbookRegistry {
    registry_url: String,
    client: reqwest::Client,
}

impl RunbookRegistry {
    pub fn new(registry_url: Option<String>) -> Self {
        Self {
            registry_url: registry_url.unwrap_or_else(|| {
                "https://raw.githubusercontent.com/deftshell/registry/main/registry.json"
                    .to_string()
            }),
            client: reqwest::Client::new(),
        }
    }

    /// Search the registry for runbooks matching a query
    pub async fn search(&self, query: &str) -> Result<Vec<RegistryEntry>> {
        let index = self.fetch_index().await?;
        let query_lower = query.to_lowercase();

        Ok(index
            .runbooks
            .into_iter()
            .filter(|entry| {
                entry.name.to_lowercase().contains(&query_lower)
                    || entry.description.to_lowercase().contains(&query_lower)
                    || entry
                        .tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(&query_lower))
            })
            .collect())
    }

    /// Get trending runbooks (sorted by stars)
    pub async fn trending(&self, limit: usize) -> Result<Vec<RegistryEntry>> {
        let mut index = self.fetch_index().await?;
        index.runbooks.sort_by_key(|b| std::cmp::Reverse(b.stars));
        index.runbooks.truncate(limit);
        Ok(index.runbooks)
    }

    /// Install a runbook by downloading it
    pub async fn install(&self, author: &str, name: &str) -> Result<super::parser::Runbook> {
        let index = self.fetch_index().await?;
        let entry = index
            .runbooks
            .iter()
            .find(|e| e.author == author && e.name == name)
            .with_context(|| format!("Runbook {}/{} not found in registry", author, name))?;

        let content = self
            .client
            .get(&entry.source_url)
            .send()
            .await?
            .text()
            .await?;

        super::parser::Runbook::parse_toml(&content)
    }

    async fn fetch_index(&self) -> Result<RegistryIndex> {
        let response = self
            .client
            .get(&self.registry_url)
            .send()
            .await
            .with_context(|| "Failed to fetch runbook registry")?;

        // If registry is not reachable, return empty index
        if !response.status().is_success() {
            return Ok(RegistryIndex {
                runbooks: Vec::new(),
            });
        }

        response
            .json()
            .await
            .with_context(|| "Failed to parse registry index")
    }
}
