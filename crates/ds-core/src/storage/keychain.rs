use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;

/// File-based credential store for DeftShell secrets (API keys, tokens, etc.).
///
/// Stores credentials in `~/.deftshell/credentials.toml` with restrictive
/// file permissions (0600 on Unix). This avoids macOS Keychain Access
/// popups that occur with the OS-native credential store for unsigned
/// binaries.
///
/// The file format is a simple TOML map of `service.key = "value"` entries:
///
/// ```toml
/// [auth]
/// anthropic_api_key = "sk-ant-..."
/// openai_api_key = "sk-..."
/// ```
pub struct KeychainStore;

impl KeychainStore {
    pub fn new() -> Self {
        Self
    }

    /// Path to the credentials file: `~/.deftshell/credentials.toml`
    fn credentials_path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".deftshell/credentials.toml"))
    }

    /// Load all credentials from the file.
    fn load_credentials() -> HashMap<String, HashMap<String, String>> {
        let Some(path) = Self::credentials_path() else {
            return HashMap::new();
        };
        if !path.exists() {
            return HashMap::new();
        }
        match std::fs::read_to_string(&path) {
            Ok(contents) => toml::from_str(&contents).unwrap_or_else(|e| {
                tracing::warn!("Failed to parse credentials file: {e}");
                HashMap::new()
            }),
            Err(e) => {
                tracing::warn!("Failed to read credentials file: {e}");
                HashMap::new()
            }
        }
    }

    /// Write all credentials to the file with restrictive permissions.
    fn save_credentials(creds: &HashMap<String, HashMap<String, String>>) -> Result<()> {
        let Some(path) = Self::credentials_path() else {
            anyhow::bail!("Could not determine home directory");
        };

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let contents = toml::to_string_pretty(creds)?;
        std::fs::write(&path, &contents)?;

        // Set file permissions to 0600 (owner read/write only) on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(&path, perms)?;
        }

        Ok(())
    }

    /// Store a secret in the credential file.
    ///
    /// `service` is a namespace (e.g., "auth"). `key` identifies the
    /// individual credential (e.g., "anthropic_api_key").
    pub fn store_secret(&self, service: &str, key: &str, value: &str) -> Result<()> {
        let mut creds = Self::load_credentials();
        creds
            .entry(service.to_string())
            .or_default()
            .insert(key.to_string(), value.to_string());
        Self::save_credentials(&creds)?;
        tracing::debug!("Stored secret {key} in credentials file (service={service})");
        Ok(())
    }

    /// Retrieve a secret from the credential file.
    ///
    /// Returns `None` if the key does not exist or the file is unavailable.
    pub fn get_secret(&self, service: &str, key: &str) -> Option<String> {
        let creds = Self::load_credentials();
        let value = creds.get(service)?.get(key)?.clone();
        if value.is_empty() {
            return None;
        }
        tracing::debug!("Retrieved secret {key} from credentials file (service={service})");
        Some(value)
    }

    /// Delete a secret from the credential file.
    ///
    /// Returns `Ok(())` even if the entry did not exist.
    pub fn delete_secret(&self, service: &str, key: &str) -> Result<()> {
        let mut creds = Self::load_credentials();
        if let Some(section) = creds.get_mut(service) {
            section.remove(key);
            // Remove the section if empty
            if section.is_empty() {
                creds.remove(service);
            }
        }
        Self::save_credentials(&creds)?;
        tracing::debug!("Deleted secret {key} from credentials file (service={service})");
        Ok(())
    }
}

impl Default for KeychainStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keychain_store_creation() {
        let _store = KeychainStore::new();
        let _store_default = KeychainStore::default();
    }

    #[test]
    fn test_credentials_path_is_some() {
        // Should return a path on any system with a home directory.
        let path = KeychainStore::credentials_path();
        assert!(path.is_some());
    }

    #[test]
    fn test_roundtrip_with_tempdir() {
        // This test uses a real temp file to verify serialization round-trip.
        let dir = tempfile::tempdir().unwrap();
        let cred_path = dir.path().join("credentials.toml");

        let mut creds: HashMap<String, HashMap<String, String>> = HashMap::new();
        creds
            .entry("auth".to_string())
            .or_default()
            .insert("test_key".to_string(), "test_value".to_string());

        let contents = toml::to_string_pretty(&creds).unwrap();
        std::fs::write(&cred_path, &contents).unwrap();

        let loaded: HashMap<String, HashMap<String, String>> =
            toml::from_str(&std::fs::read_to_string(&cred_path).unwrap()).unwrap();
        assert_eq!(
            loaded.get("auth").unwrap().get("test_key").unwrap(),
            "test_value"
        );
    }
}
