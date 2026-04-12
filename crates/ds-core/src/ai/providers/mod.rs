pub mod anthropic;
pub mod bedrock;
pub mod copilot;
pub mod gemini;
pub mod ollama;
pub mod openai;

use crate::storage::KeychainStore;

/// Resolve an API key by checking (in order):
///   1. Environment variable (`env_var_name`)
///   2. Credential store (`keychain_key`, stored by `ds auth <provider>`)
///
/// Returns `Ok(key)` if found, or a helpful error message if not.
pub fn resolve_api_key(
    provider_name: &str,
    env_var_name: &str,
    keychain_key: &str,
) -> Result<String, anyhow::Error> {
    // 1. Check environment variable first (takes priority).
    if let Ok(key) = std::env::var(env_var_name) {
        if !key.is_empty() {
            return Ok(key);
        }
    }

    // 2. Check credential store (stored by `ds auth <provider>`).
    let keychain = KeychainStore::new();
    if let Some(key) = keychain.get_secret("auth", keychain_key) {
        if !key.is_empty() {
            return Ok(key);
        }
    }

    Err(anyhow::anyhow!(
        "{provider_name} API key not found.\n\n\
         Set it up with one of:\n  \
         - ds auth {provider_name}\n  \
         - export {env_var_name}=<your-api-key>"
    ))
}
