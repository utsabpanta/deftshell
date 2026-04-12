use std::path::{Path, PathBuf};

/// The kind of workspace/monorepo tool detected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceType {
    Nx,
    Turborepo,
    Lerna,
    Yarn,
    Pnpm,
    Cargo,
    Npm,
}

impl std::fmt::Display for WorkspaceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkspaceType::Nx => write!(f, "nx"),
            WorkspaceType::Turborepo => write!(f, "turborepo"),
            WorkspaceType::Lerna => write!(f, "lerna"),
            WorkspaceType::Yarn => write!(f, "yarn"),
            WorkspaceType::Pnpm => write!(f, "pnpm"),
            WorkspaceType::Cargo => write!(f, "cargo"),
            WorkspaceType::Npm => write!(f, "npm"),
        }
    }
}

/// Metadata about a detected monorepo/workspace.
#[derive(Debug, Clone)]
pub struct WorkspaceInfo {
    /// Absolute path to the workspace root.
    pub root: PathBuf,
    /// The tool that manages the workspace.
    pub workspace_type: WorkspaceType,
    /// Glob patterns defining where packages live (e.g. `["packages/*"]`).
    pub package_globs: Vec<String>,
}

/// A single package inside a workspace.
#[derive(Debug, Clone)]
pub struct WorkspacePackage {
    /// The package name (from its manifest).
    pub name: String,
    /// Absolute path to the package directory.
    pub path: PathBuf,
}

/// Detect workspace/monorepo configuration in the given directory.
///
/// Returns `None` if the directory is not a workspace root.
pub fn detect_workspace(dir: &Path) -> Option<WorkspaceInfo> {
    // Try each detector in order of specificity.
    // Nx and Turborepo are specialized monorepo tools and take priority.
    if let Some(info) = detect_nx(dir) {
        return Some(info);
    }
    if let Some(info) = detect_turborepo(dir) {
        return Some(info);
    }
    if let Some(info) = detect_lerna(dir) {
        return Some(info);
    }
    if let Some(info) = detect_pnpm_workspace(dir) {
        return Some(info);
    }
    if let Some(info) = detect_npm_or_yarn_workspaces(dir) {
        return Some(info);
    }
    if let Some(info) = detect_cargo_workspace(dir) {
        return Some(info);
    }
    None
}

/// List all packages inside a workspace by expanding the glob patterns.
pub fn list_workspace_packages(info: &WorkspaceInfo) -> Vec<WorkspacePackage> {
    let mut packages = Vec::new();

    for glob_pattern in &info.package_globs {
        let full_pattern = info.root.join(glob_pattern).display().to_string();
        if let Ok(entries) = glob::glob(&full_pattern) {
            for entry in entries.flatten() {
                if !entry.is_dir() {
                    continue;
                }
                let name =
                    extract_package_name(&entry, &info.workspace_type).unwrap_or_else(|| {
                        entry
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string()
                    });
                packages.push(WorkspacePackage { name, path: entry });
            }
        }
    }

    packages.sort_by(|a, b| a.name.cmp(&b.name));
    packages.dedup_by(|a, b| a.path == b.path);
    packages
}

// ── Individual workspace detectors ──────────────────────────────────────

fn detect_nx(dir: &Path) -> Option<WorkspaceInfo> {
    let nx_json = dir.join("nx.json");
    if !nx_json.exists() {
        return None;
    }

    // Nx projects can live in multiple directories. Typical defaults:
    let mut globs = vec![
        "packages/*".to_string(),
        "apps/*".to_string(),
        "libs/*".to_string(),
    ];

    // Try to read workspace layout from nx.json
    if let Ok(contents) = std::fs::read_to_string(&nx_json) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&contents) {
            // nx.json may specify workspaceLayout.appsDir / libsDir
            let apps_dir = json
                .pointer("/workspaceLayout/appsDir")
                .and_then(|v| v.as_str())
                .unwrap_or("apps");
            let libs_dir = json
                .pointer("/workspaceLayout/libsDir")
                .and_then(|v| v.as_str())
                .unwrap_or("libs");
            globs = vec![
                format!("{apps_dir}/*"),
                format!("{libs_dir}/*"),
                "packages/*".to_string(),
            ];
        }
    }

    Some(WorkspaceInfo {
        root: dir.to_path_buf(),
        workspace_type: WorkspaceType::Nx,
        package_globs: globs,
    })
}

fn detect_turborepo(dir: &Path) -> Option<WorkspaceInfo> {
    let turbo_json = dir.join("turbo.json");
    if !turbo_json.exists() {
        return None;
    }

    // Turborepo relies on the underlying package manager's workspace config.
    // Try to read workspaces from package.json.
    let globs = read_package_json_workspaces(dir)
        .unwrap_or_else(|| vec!["packages/*".to_string(), "apps/*".to_string()]);

    Some(WorkspaceInfo {
        root: dir.to_path_buf(),
        workspace_type: WorkspaceType::Turborepo,
        package_globs: globs,
    })
}

fn detect_lerna(dir: &Path) -> Option<WorkspaceInfo> {
    let lerna_json = dir.join("lerna.json");
    if !lerna_json.exists() {
        return None;
    }

    let mut globs = vec!["packages/*".to_string()];

    if let Ok(contents) = std::fs::read_to_string(&lerna_json) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&contents) {
            if let Some(packages) = json.get("packages").and_then(|v| v.as_array()) {
                let parsed: Vec<String> = packages
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
                if !parsed.is_empty() {
                    globs = parsed;
                }
            }
        }
    }

    Some(WorkspaceInfo {
        root: dir.to_path_buf(),
        workspace_type: WorkspaceType::Lerna,
        package_globs: globs,
    })
}

fn detect_pnpm_workspace(dir: &Path) -> Option<WorkspaceInfo> {
    let pnpm_ws = dir.join("pnpm-workspace.yaml");
    if !pnpm_ws.exists() {
        return None;
    }

    // pnpm-workspace.yaml is a simple YAML file with a `packages` list.
    // We do a lightweight parse to avoid adding a full YAML dependency.
    let mut globs = vec!["packages/*".to_string()];
    if let Ok(contents) = std::fs::read_to_string(&pnpm_ws) {
        let parsed = parse_pnpm_workspace_yaml(&contents);
        if !parsed.is_empty() {
            globs = parsed;
        }
    }

    Some(WorkspaceInfo {
        root: dir.to_path_buf(),
        workspace_type: WorkspaceType::Pnpm,
        package_globs: globs,
    })
}

fn detect_npm_or_yarn_workspaces(dir: &Path) -> Option<WorkspaceInfo> {
    let globs = read_package_json_workspaces(dir)?;

    // Determine if Yarn or npm based on lock file
    let workspace_type = if dir.join("yarn.lock").exists() {
        WorkspaceType::Yarn
    } else {
        WorkspaceType::Npm
    };

    Some(WorkspaceInfo {
        root: dir.to_path_buf(),
        workspace_type,
        package_globs: globs,
    })
}

fn detect_cargo_workspace(dir: &Path) -> Option<WorkspaceInfo> {
    let cargo_toml = dir.join("Cargo.toml");
    if !cargo_toml.exists() {
        return None;
    }

    let contents = std::fs::read_to_string(&cargo_toml).ok()?;
    let parsed: toml::Value = toml::from_str(&contents).ok()?;

    let workspace = parsed.get("workspace")?;
    let members = workspace.get("members")?.as_array()?;

    let globs: Vec<String> = members
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();

    if globs.is_empty() {
        return None;
    }

    Some(WorkspaceInfo {
        root: dir.to_path_buf(),
        workspace_type: WorkspaceType::Cargo,
        package_globs: globs,
    })
}

// ── Helpers ─────────────────────────────────────────────────────────────

/// Read the `workspaces` field from package.json.
///
/// Handles both the array form and the `{ packages: [...] }` object form.
fn read_package_json_workspaces(dir: &Path) -> Option<Vec<String>> {
    let path = dir.join("package.json");
    let contents = std::fs::read_to_string(&path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&contents).ok()?;

    let workspaces = json.get("workspaces")?;

    let arr = if let Some(arr) = workspaces.as_array() {
        arr.clone()
    } else if let Some(arr) = workspaces.get("packages").and_then(|v| v.as_array()) {
        arr.clone()
    } else {
        return None;
    };

    let globs: Vec<String> = arr
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();

    if globs.is_empty() {
        None
    } else {
        Some(globs)
    }
}

/// Lightweight parser for pnpm-workspace.yaml.
///
/// Extracts the `packages:` list without requiring a full YAML parser.
/// Handles the common format:
/// ```yaml
/// packages:
///   - 'packages/*'
///   - 'apps/*'
/// ```
fn parse_pnpm_workspace_yaml(contents: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut in_packages = false;

    for line in contents.lines() {
        let trimmed = line.trim();

        if trimmed == "packages:" || trimmed.starts_with("packages:") {
            in_packages = true;
            // Check for inline value (shouldn't happen normally, but be safe)
            continue;
        }

        if in_packages {
            // End of list when we hit a non-list-item, non-empty, non-comment line
            if !trimmed.is_empty() && !trimmed.starts_with('-') && !trimmed.starts_with('#') {
                break;
            }
            if let Some(item) = trimmed.strip_prefix('-') {
                let item = item.trim().trim_matches('\'').trim_matches('"');
                if !item.is_empty() {
                    result.push(item.to_string());
                }
            }
        }
    }

    result
}

/// Extract a package name from its manifest file.
fn extract_package_name(pkg_dir: &Path, ws_type: &WorkspaceType) -> Option<String> {
    match ws_type {
        WorkspaceType::Cargo => {
            let cargo_toml = pkg_dir.join("Cargo.toml");
            let contents = std::fs::read_to_string(&cargo_toml).ok()?;
            let parsed: toml::Value = toml::from_str(&contents).ok()?;
            parsed
                .get("package")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
                .map(String::from)
        }
        _ => {
            // JavaScript-family: read package.json
            let pkg_json = pkg_dir.join("package.json");
            let contents = std::fs::read_to_string(&pkg_json).ok()?;
            let parsed: serde_json::Value = serde_json::from_str(&contents).ok()?;
            parsed
                .get("name")
                .and_then(|n| n.as_str())
                .map(String::from)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_dir() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn test_detect_no_workspace() {
        let dir = setup_dir();
        assert!(detect_workspace(dir.path()).is_none());
    }

    #[test]
    fn test_detect_nx_workspace() {
        let dir = setup_dir();
        std::fs::write(
            dir.path().join("nx.json"),
            r#"{"workspaceLayout":{"appsDir":"apps","libsDir":"libs"}}"#,
        )
        .unwrap();

        let info = detect_workspace(dir.path()).unwrap();
        assert_eq!(info.workspace_type, WorkspaceType::Nx);
        assert!(info.package_globs.contains(&"apps/*".to_string()));
        assert!(info.package_globs.contains(&"libs/*".to_string()));
    }

    #[test]
    fn test_detect_turborepo_workspace() {
        let dir = setup_dir();
        std::fs::write(dir.path().join("turbo.json"), "{}").unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"workspaces":["packages/*","apps/*"]}"#,
        )
        .unwrap();

        let info = detect_workspace(dir.path()).unwrap();
        assert_eq!(info.workspace_type, WorkspaceType::Turborepo);
        assert!(info.package_globs.contains(&"packages/*".to_string()));
        assert!(info.package_globs.contains(&"apps/*".to_string()));
    }

    #[test]
    fn test_detect_lerna_workspace() {
        let dir = setup_dir();
        std::fs::write(
            dir.path().join("lerna.json"),
            r#"{"packages":["modules/*"]}"#,
        )
        .unwrap();

        let info = detect_workspace(dir.path()).unwrap();
        assert_eq!(info.workspace_type, WorkspaceType::Lerna);
        assert_eq!(info.package_globs, vec!["modules/*".to_string()]);
    }

    #[test]
    fn test_detect_pnpm_workspace() {
        let dir = setup_dir();
        std::fs::write(
            dir.path().join("pnpm-workspace.yaml"),
            "packages:\n  - 'packages/*'\n  - 'tools/*'\n",
        )
        .unwrap();

        let info = detect_workspace(dir.path()).unwrap();
        assert_eq!(info.workspace_type, WorkspaceType::Pnpm);
        assert_eq!(
            info.package_globs,
            vec!["packages/*".to_string(), "tools/*".to_string()]
        );
    }

    #[test]
    fn test_detect_yarn_workspaces() {
        let dir = setup_dir();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"workspaces":["packages/*"]}"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("yarn.lock"), "").unwrap();

        let info = detect_workspace(dir.path()).unwrap();
        assert_eq!(info.workspace_type, WorkspaceType::Yarn);
    }

    #[test]
    fn test_detect_npm_workspaces() {
        let dir = setup_dir();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"workspaces":{"packages":["packages/*"]}}"#,
        )
        .unwrap();

        let info = detect_workspace(dir.path()).unwrap();
        assert_eq!(info.workspace_type, WorkspaceType::Npm);
        assert_eq!(info.package_globs, vec!["packages/*".to_string()]);
    }

    #[test]
    fn test_detect_cargo_workspace() {
        let dir = setup_dir();
        let cargo_toml = r#"
[workspace]
members = ["crates/*"]
"#;
        std::fs::write(dir.path().join("Cargo.toml"), cargo_toml).unwrap();

        let info = detect_workspace(dir.path()).unwrap();
        assert_eq!(info.workspace_type, WorkspaceType::Cargo);
        assert_eq!(info.package_globs, vec!["crates/*".to_string()]);
    }

    #[test]
    fn test_parse_pnpm_workspace_yaml() {
        let yaml = "packages:\n  - 'packages/*'\n  - \"apps/*\"\n  - tools/*\n";
        let result = parse_pnpm_workspace_yaml(yaml);
        assert_eq!(
            result,
            vec![
                "packages/*".to_string(),
                "apps/*".to_string(),
                "tools/*".to_string(),
            ]
        );
    }

    #[test]
    fn test_list_workspace_packages_cargo() {
        let dir = setup_dir();
        let root = dir.path();

        // Create workspace Cargo.toml
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/*\"]\n",
        )
        .unwrap();

        // Create two member crates
        std::fs::create_dir_all(root.join("crates/alpha")).unwrap();
        std::fs::write(
            root.join("crates/alpha/Cargo.toml"),
            "[package]\nname = \"alpha\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        std::fs::create_dir_all(root.join("crates/beta")).unwrap();
        std::fs::write(
            root.join("crates/beta/Cargo.toml"),
            "[package]\nname = \"beta\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        let info = detect_workspace(root).unwrap();
        let packages = list_workspace_packages(&info);

        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].name, "alpha");
        assert_eq!(packages[1].name, "beta");
    }
}
