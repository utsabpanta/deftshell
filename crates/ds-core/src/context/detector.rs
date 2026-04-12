use anyhow::{Context, Result};
use std::path::Path;

use super::stack_profile::StackProfile;
use crate::config::loader::ConfigLoader;

/// Navigate a TOML value using a JSON-pointer-like path (e.g. "/tool/poetry").
fn toml_pointer<'a>(value: &'a toml::Value, path: &str) -> Option<&'a toml::Value> {
    let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    let mut current = value;
    for part in parts {
        current = current.get(part)?;
    }
    Some(current)
}

/// Detection source files used for cache invalidation.
/// Listed roughly in priority order.
pub const DETECTION_SOURCES: &[&str] = &[
    ".deftshell.toml",
    "package.json",
    "Cargo.toml",
    "pyproject.toml",
    "go.mod",
    "Gemfile",
    "pom.xml",
    "build.gradle",
    "composer.json",
    "mix.exs",
    "pubspec.yaml",
    "Package.swift",
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "Cargo.lock",
    "poetry.lock",
    "go.sum",
    "Gemfile.lock",
    "tsconfig.json",
    "Dockerfile",
    "docker-compose.yml",
    "docker-compose.yaml",
    "Makefile",
    ".env",
    ".env.local",
    ".nvmrc",
    ".node-version",
    ".python-version",
    ".ruby-version",
    ".tool-versions",
    "rust-toolchain.toml",
    ".gitlab-ci.yml",
    "Jenkinsfile",
    "azure-pipelines.yml",
    "serverless.yml",
    "nx.json",
    "turbo.json",
    "lerna.json",
    "pnpm-workspace.yaml",
];

/// The main context detection engine. Scans a project directory and builds
/// a [`StackProfile`] describing the project's technology stack.
pub struct ContextDetector;

impl ContextDetector {
    /// Detect the full stack profile for a given directory.
    pub fn detect(dir: &Path) -> Result<StackProfile> {
        let dir = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());

        let mut profile = StackProfile::default();
        profile.project.root = dir.display().to_string();

        // Detection sources in priority order:
        // 1. Explicit .deftshell.toml config
        Self::detect_deftshell_config(&dir, &mut profile)?;
        // 2. Package manifests
        Self::detect_package_manifests(&dir, &mut profile)?;
        // 3. Lock files (package manager detection)
        Self::detect_lock_files(&dir, &mut profile);
        // 4. Config files
        Self::detect_config_files(&dir, &mut profile);
        // 5. Directory structure
        Self::detect_directory_structure(&dir, &mut profile);
        // 6. Environment files
        Self::detect_env_files(&dir, &mut profile);
        // 7. VCS
        Self::detect_vcs(&dir, &mut profile);
        // 8. Runtime versions
        Self::detect_runtime_versions(&dir, &mut profile);
        // 9. CI/CD
        Self::detect_ci_cd(&dir, &mut profile);
        // 10. Cloud provider
        Self::detect_cloud_provider(&dir, &mut profile);
        // 11. Docker / compose
        Self::detect_docker(&dir, &mut profile);
        // 12. Services from compose / env
        Self::detect_services(&dir, &mut profile);

        Ok(profile)
    }

    // ── 1. Explicit .deftshell.toml ──────────────────────────────────────

    fn detect_deftshell_config(dir: &Path, profile: &mut StackProfile) -> Result<()> {
        if let Ok(Some(project_config)) = ConfigLoader::load_project_config(dir) {
            if let Some(name) = &project_config.project.name {
                profile.project.name = name.clone();
            }

            let overrides = &project_config.stack;
            if overrides.primary_language.is_some() {
                profile.stack.primary_language = overrides.primary_language.clone();
            }
            if overrides.runtime.is_some() {
                profile.stack.runtime = overrides.runtime.clone();
            }
            if overrides.framework.is_some() {
                profile.stack.framework = overrides.framework.clone();
            }
            if overrides.test_runner.is_some() {
                profile.stack.test_runner = overrides.test_runner.clone();
            }
            if overrides.linter.is_some() {
                profile.stack.linter = overrides.linter.clone();
            }
            if overrides.formatter.is_some() {
                profile.stack.formatter = overrides.formatter.clone();
            }
            if overrides.bundler.is_some() {
                profile.stack.bundler = overrides.bundler.clone();
            }
            if overrides.package_manager.is_some() {
                profile.stack.package_manager = overrides.package_manager.clone();
            }

            profile.scripts = project_config.scripts;
        }
        Ok(())
    }

    // ── 2. Package manifests ────────────────────────────────────────────

    fn detect_package_manifests(dir: &Path, profile: &mut StackProfile) -> Result<()> {
        Self::detect_package_json(dir, profile)?;
        Self::detect_cargo_toml(dir, profile)?;
        Self::detect_pyproject_toml(dir, profile)?;
        Self::detect_go_mod(dir, profile)?;
        Self::detect_gemfile(dir, profile);
        Self::detect_pom_xml(dir, profile);
        Self::detect_build_gradle(dir, profile);
        Self::detect_composer_json(dir, profile)?;
        Self::detect_mix_exs(dir, profile);
        Self::detect_pubspec_yaml(dir, profile);
        Self::detect_package_swift(dir, profile);
        Ok(())
    }

    fn detect_package_json(dir: &Path, profile: &mut StackProfile) -> Result<()> {
        let path = dir.join("package.json");
        if !path.exists() {
            return Ok(());
        }

        let contents = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Could not read {}: {}", path.display(), e);
                return Ok(());
            }
        };
        let json: serde_json::Value = match serde_json::from_str(&contents) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Could not parse {}: {}", path.display(), e);
                return Ok(());
            }
        };

        // Project name
        if profile.project.name.is_empty() {
            if let Some(name) = json.get("name").and_then(|v| v.as_str()) {
                profile.project.name = name.to_string();
            }
        }

        // Primary language
        if profile.stack.primary_language.is_none() {
            // Check for TypeScript
            let has_ts = dir.join("tsconfig.json").exists()
                || json.pointer("/devDependencies/typescript").is_some()
                || json.pointer("/dependencies/typescript").is_some();
            if has_ts {
                profile.stack.primary_language = Some("typescript".to_string());
            } else {
                profile.stack.primary_language = Some("javascript".to_string());
            }
        }

        // Runtime
        if profile.stack.runtime.is_none() {
            profile.stack.runtime = Some("node".to_string());
        }

        // Scripts
        if profile.scripts.is_empty() {
            if let Some(scripts) = json.get("scripts").and_then(|v| v.as_object()) {
                for (key, value) in scripts {
                    if let Some(cmd) = value.as_str() {
                        profile.scripts.insert(key.clone(), cmd.to_string());
                    }
                }
            }
        }

        // Framework detection from dependencies
        if profile.stack.framework.is_none() {
            profile.stack.framework = Self::detect_js_framework(&json);
        }

        // Framework version
        if profile.stack.framework.is_some() && profile.stack.framework_version.is_none() {
            let fw = profile.stack.framework.as_deref().unwrap_or_default();
            let dep_key = match fw {
                "next" => "next",
                "react" => "react",
                "vue" => "vue",
                "angular" => "@angular/core",
                "svelte" => "svelte",
                "nuxt" => "nuxt",
                "remix" => "@remix-run/react",
                "astro" => "astro",
                "gatsby" => "gatsby",
                "express" => "express",
                "nestjs" => "@nestjs/core",
                "fastify" => "fastify",
                _ => "",
            };
            if !dep_key.is_empty() {
                profile.stack.framework_version = Self::extract_dep_version(&json, dep_key);
            }
        }

        // Test runner detection
        if profile.stack.test_runner.is_none() {
            profile.stack.test_runner = Self::detect_js_test_runner(&json);
        }

        // Linter detection
        if profile.stack.linter.is_none() {
            if Self::has_dep(&json, "eslint")
                || dir.join(".eslintrc.json").exists()
                || dir.join(".eslintrc.js").exists()
                || dir.join(".eslintrc.yml").exists()
                || dir.join("eslint.config.js").exists()
                || dir.join("eslint.config.mjs").exists()
            {
                profile.stack.linter = Some("eslint".to_string());
            } else if Self::has_dep(&json, "biome")
                || Self::has_dep(&json, "@biomejs/biome")
                || dir.join("biome.json").exists()
            {
                profile.stack.linter = Some("biome".to_string());
            }
        }

        // Formatter detection
        if profile.stack.formatter.is_none() {
            if Self::has_dep(&json, "prettier")
                || dir.join(".prettierrc").exists()
                || dir.join(".prettierrc.json").exists()
                || dir.join("prettier.config.js").exists()
            {
                profile.stack.formatter = Some("prettier".to_string());
            } else if Self::has_dep(&json, "biome") || Self::has_dep(&json, "@biomejs/biome") {
                profile.stack.formatter = Some("biome".to_string());
            }
        }

        // Bundler detection
        if profile.stack.bundler.is_none() {
            if Self::has_dep(&json, "vite") {
                profile.stack.bundler = Some("vite".to_string());
            } else if Self::has_dep(&json, "webpack") {
                profile.stack.bundler = Some("webpack".to_string());
            } else if Self::has_dep(&json, "esbuild") {
                profile.stack.bundler = Some("esbuild".to_string());
            } else if Self::has_dep(&json, "rollup") {
                profile.stack.bundler = Some("rollup".to_string());
            } else if Self::has_dep(&json, "parcel") || Self::has_dep(&json, "@parcel/core") {
                profile.stack.bundler = Some("parcel".to_string());
            } else if Self::has_dep(&json, "turbopack") {
                profile.stack.bundler = Some("turbopack".to_string());
            }
        }

        Ok(())
    }

    fn detect_js_framework(json: &serde_json::Value) -> Option<String> {
        // Order matters: more specific frameworks first
        let frameworks = [
            ("next", "next"),
            ("nuxt", "nuxt"),
            ("remix", "@remix-run/react"),
            ("astro", "astro"),
            ("gatsby", "gatsby"),
            ("svelte", "svelte"),
            ("angular", "@angular/core"),
            ("vue", "vue"),
            ("nestjs", "@nestjs/core"),
            ("fastify", "fastify"),
            ("express", "express"),
            ("react", "react"),
        ];
        for (name, dep) in &frameworks {
            if Self::has_dep(json, dep) {
                return Some(name.to_string());
            }
        }
        None
    }

    fn detect_js_test_runner(json: &serde_json::Value) -> Option<String> {
        let runners = [
            ("vitest", "vitest"),
            ("jest", "jest"),
            ("mocha", "mocha"),
            ("ava", "ava"),
            ("jasmine", "jasmine"),
            ("playwright", "@playwright/test"),
            ("cypress", "cypress"),
        ];
        for (name, dep) in &runners {
            if Self::has_dep(json, dep) {
                return Some(name.to_string());
            }
        }
        None
    }

    fn has_dep(json: &serde_json::Value, name: &str) -> bool {
        json.pointer(&format!("/dependencies/{name}")).is_some()
            || json.pointer(&format!("/devDependencies/{name}")).is_some()
            || json.pointer(&format!("/peerDependencies/{name}")).is_some()
    }

    fn extract_dep_version(json: &serde_json::Value, name: &str) -> Option<String> {
        json.pointer(&format!("/dependencies/{name}"))
            .or_else(|| json.pointer(&format!("/devDependencies/{name}")))
            .and_then(|v| v.as_str())
            .map(|s| {
                s.trim_start_matches(|c: char| !c.is_ascii_digit())
                    .to_string()
            })
    }

    fn detect_cargo_toml(dir: &Path, profile: &mut StackProfile) -> Result<()> {
        let path = dir.join("Cargo.toml");
        if !path.exists() {
            return Ok(());
        }

        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let cargo: toml::Value = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse {}", path.display()))?;

        // Project name
        if profile.project.name.is_empty() {
            if let Some(name) = cargo
                .get("package")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
            {
                profile.project.name = name.to_string();
            }
        }

        // Primary language
        if profile.stack.primary_language.is_none() {
            profile.stack.primary_language = Some("rust".to_string());
        }

        // Runtime
        if profile.stack.runtime.is_none() {
            profile.stack.runtime = Some("rust".to_string());
        }

        // Package manager
        if profile.stack.package_manager.is_none() {
            profile.stack.package_manager = Some("cargo".to_string());
        }

        // Detect workspace
        if cargo.get("workspace").is_some() {
            // Workspace root detected; name comes from workspace if no package
            if profile.project.name.is_empty() {
                // Use the directory name as project name for workspaces
                if let Some(dir_name) = dir.file_name().and_then(|n| n.to_str()) {
                    profile.project.name = dir_name.to_string();
                }
            }
        }

        // Detect common frameworks from dependencies
        if profile.stack.framework.is_none() {
            let deps = cargo.get("dependencies");
            if let Some(deps) = deps {
                if deps.get("actix-web").is_some() {
                    profile.stack.framework = Some("actix-web".to_string());
                } else if deps.get("axum").is_some() {
                    profile.stack.framework = Some("axum".to_string());
                } else if deps.get("rocket").is_some() {
                    profile.stack.framework = Some("rocket".to_string());
                } else if deps.get("warp").is_some() {
                    profile.stack.framework = Some("warp".to_string());
                } else if deps.get("tauri").is_some() {
                    profile.stack.framework = Some("tauri".to_string());
                } else if deps.get("leptos").is_some() {
                    profile.stack.framework = Some("leptos".to_string());
                } else if deps.get("yew").is_some() {
                    profile.stack.framework = Some("yew".to_string());
                }
            }
        }

        Ok(())
    }

    fn detect_pyproject_toml(dir: &Path, profile: &mut StackProfile) -> Result<()> {
        let path = dir.join("pyproject.toml");
        if !path.exists() {
            return Ok(());
        }

        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let pyproject: toml::Value = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse {}", path.display()))?;

        // Project name
        if profile.project.name.is_empty() {
            if let Some(name) = pyproject
                .get("project")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
            {
                profile.project.name = name.to_string();
            } else if let Some(name) = pyproject
                .get("tool")
                .and_then(|t| t.get("poetry"))
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
            {
                profile.project.name = name.to_string();
            }
        }

        // Primary language
        if profile.stack.primary_language.is_none() {
            profile.stack.primary_language = Some("python".to_string());
        }

        // Runtime
        if profile.stack.runtime.is_none() {
            profile.stack.runtime = Some("python".to_string());
        }

        // Package manager: poetry vs pip vs pdm vs hatch
        if profile.stack.package_manager.is_none() {
            let build_backend = pyproject
                .get("build-system")
                .and_then(|bs| bs.get("build-backend"))
                .and_then(|b| b.as_str())
                .unwrap_or_default();
            if build_backend.contains("poetry")
                || toml_pointer(&pyproject, "/tool/poetry").is_some()
            {
                profile.stack.package_manager = Some("poetry".to_string());
            } else if build_backend.contains("pdm")
                || toml_pointer(&pyproject, "/tool/pdm").is_some()
            {
                profile.stack.package_manager = Some("pdm".to_string());
            } else if build_backend.contains("hatch")
                || toml_pointer(&pyproject, "/tool/hatch").is_some()
            {
                profile.stack.package_manager = Some("hatch".to_string());
            } else if dir.join("requirements.txt").exists() || dir.join("setup.py").exists() {
                profile.stack.package_manager = Some("pip".to_string());
            } else if dir.join("Pipfile").exists() {
                profile.stack.package_manager = Some("pipenv".to_string());
            } else if dir.join("uv.lock").exists() || toml_pointer(&pyproject, "/tool/uv").is_some()
            {
                profile.stack.package_manager = Some("uv".to_string());
            }
        }

        // Framework detection from dependencies
        if profile.stack.framework.is_none() {
            let all_deps = Self::collect_python_deps(&pyproject);
            if all_deps.iter().any(|d| d.starts_with("django")) {
                profile.stack.framework = Some("django".to_string());
            } else if all_deps.iter().any(|d| d.starts_with("fastapi")) {
                profile.stack.framework = Some("fastapi".to_string());
            } else if all_deps.iter().any(|d| d.starts_with("flask")) {
                profile.stack.framework = Some("flask".to_string());
            } else if all_deps.iter().any(|d| d.starts_with("starlette")) {
                profile.stack.framework = Some("starlette".to_string());
            } else if all_deps.iter().any(|d| d.starts_with("tornado")) {
                profile.stack.framework = Some("tornado".to_string());
            }
        }

        // Test runner
        if profile.stack.test_runner.is_none()
            && (toml_pointer(&pyproject, "/tool/pytest").is_some()
                || toml_pointer(&pyproject, "/tool/pytest/ini_options").is_some())
        {
            profile.stack.test_runner = Some("pytest".to_string());
        }

        // Linter
        if profile.stack.linter.is_none() {
            if toml_pointer(&pyproject, "/tool/ruff").is_some() || dir.join("ruff.toml").exists() {
                profile.stack.linter = Some("ruff".to_string());
            } else if toml_pointer(&pyproject, "/tool/flake8").is_some() {
                profile.stack.linter = Some("flake8".to_string());
            } else if toml_pointer(&pyproject, "/tool/pylint").is_some() {
                profile.stack.linter = Some("pylint".to_string());
            }
        }

        // Formatter
        if profile.stack.formatter.is_none() {
            if toml_pointer(&pyproject, "/tool/black").is_some() {
                profile.stack.formatter = Some("black".to_string());
            } else if toml_pointer(&pyproject, "/tool/ruff/format").is_some() {
                profile.stack.formatter = Some("ruff".to_string());
            }
        }

        Ok(())
    }

    /// Collect all dependency names from a pyproject.toml (PEP 621 + poetry).
    fn collect_python_deps(pyproject: &toml::Value) -> Vec<String> {
        let mut deps = Vec::new();
        // PEP 621: project.dependencies
        if let Some(arr) =
            toml_pointer(pyproject, "/project/dependencies").and_then(|v| v.as_array())
        {
            for item in arr {
                if let Some(s) = item.as_str() {
                    // "django>=4.0" -> "django"
                    let name = s
                        .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
                        .next()
                        .unwrap_or_default()
                        .to_lowercase();
                    deps.push(name);
                }
            }
        }
        // Poetry: tool.poetry.dependencies
        if let Some(table) =
            toml_pointer(pyproject, "/tool/poetry/dependencies").and_then(|v| v.as_table())
        {
            for key in table.keys() {
                let name: String = key.to_lowercase();
                deps.push(name);
            }
        }
        deps
    }

    fn detect_go_mod(dir: &Path, profile: &mut StackProfile) -> Result<()> {
        let path = dir.join("go.mod");
        if !path.exists() {
            return Ok(());
        }

        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;

        // Extract module name from the first "module" line
        if profile.project.name.is_empty() {
            for line in contents.lines() {
                let trimmed = line.trim();
                if let Some(module_path) = trimmed.strip_prefix("module ") {
                    let module_path = module_path.trim();
                    // Use the last path segment as the project name
                    let name = module_path.rsplit('/').next().unwrap_or(module_path);
                    profile.project.name = name.to_string();
                    break;
                }
            }
        }

        if profile.stack.primary_language.is_none() {
            profile.stack.primary_language = Some("go".to_string());
        }
        if profile.stack.runtime.is_none() {
            profile.stack.runtime = Some("go".to_string());
        }
        if profile.stack.package_manager.is_none() {
            profile.stack.package_manager = Some("go modules".to_string());
        }

        // Detect Go runtime version from the go directive
        if profile.stack.runtime_version.is_none() {
            for line in contents.lines() {
                let trimmed = line.trim();
                if let Some(version) = trimmed.strip_prefix("go ") {
                    profile.stack.runtime_version = Some(version.trim().to_string());
                    break;
                }
            }
        }

        Ok(())
    }

    fn detect_gemfile(dir: &Path, profile: &mut StackProfile) {
        if !dir.join("Gemfile").exists() {
            return;
        }
        if profile.stack.primary_language.is_none() {
            profile.stack.primary_language = Some("ruby".to_string());
        }
        if profile.stack.runtime.is_none() {
            profile.stack.runtime = Some("ruby".to_string());
        }
        if profile.stack.package_manager.is_none() {
            profile.stack.package_manager = Some("bundler".to_string());
        }
        // Simple framework detection by reading Gemfile
        if profile.stack.framework.is_none() {
            if let Ok(contents) = std::fs::read_to_string(dir.join("Gemfile")) {
                if contents.contains("'rails'") || contents.contains("\"rails\"") {
                    profile.stack.framework = Some("rails".to_string());
                } else if contents.contains("'sinatra'") || contents.contains("\"sinatra\"") {
                    profile.stack.framework = Some("sinatra".to_string());
                } else if contents.contains("'hanami'") || contents.contains("\"hanami\"") {
                    profile.stack.framework = Some("hanami".to_string());
                }
            }
        }
    }

    fn detect_pom_xml(dir: &Path, profile: &mut StackProfile) {
        if !dir.join("pom.xml").exists() {
            return;
        }
        if profile.stack.primary_language.is_none() {
            profile.stack.primary_language = Some("java".to_string());
        }
        if profile.stack.runtime.is_none() {
            profile.stack.runtime = Some("jvm".to_string());
        }
        if profile.stack.package_manager.is_none() {
            profile.stack.package_manager = Some("maven".to_string());
        }
    }

    fn detect_build_gradle(dir: &Path, profile: &mut StackProfile) {
        if !dir.join("build.gradle").exists() && !dir.join("build.gradle.kts").exists() {
            return;
        }
        if profile.stack.primary_language.is_none() {
            if dir.join("build.gradle.kts").exists() {
                profile.stack.primary_language = Some("kotlin".to_string());
            } else {
                profile.stack.primary_language = Some("java".to_string());
            }
        }
        if profile.stack.runtime.is_none() {
            profile.stack.runtime = Some("jvm".to_string());
        }
        if profile.stack.package_manager.is_none() {
            profile.stack.package_manager = Some("gradle".to_string());
        }
    }

    fn detect_composer_json(dir: &Path, profile: &mut StackProfile) -> Result<()> {
        let path = dir.join("composer.json");
        if !path.exists() {
            return Ok(());
        }

        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let json: serde_json::Value = serde_json::from_str(&contents)
            .with_context(|| format!("Failed to parse {}", path.display()))?;

        if profile.project.name.is_empty() {
            if let Some(name) = json.get("name").and_then(|v| v.as_str()) {
                profile.project.name = name.to_string();
            }
        }

        if profile.stack.primary_language.is_none() {
            profile.stack.primary_language = Some("php".to_string());
        }
        if profile.stack.runtime.is_none() {
            profile.stack.runtime = Some("php".to_string());
        }
        if profile.stack.package_manager.is_none() {
            profile.stack.package_manager = Some("composer".to_string());
        }

        // Framework detection
        if profile.stack.framework.is_none() {
            if json.pointer("/require/laravel/framework").is_some() {
                profile.stack.framework = Some("laravel".to_string());
            } else if json.pointer("/require/symfony/framework-bundle").is_some() {
                profile.stack.framework = Some("symfony".to_string());
            }
        }

        Ok(())
    }

    fn detect_mix_exs(dir: &Path, profile: &mut StackProfile) {
        if !dir.join("mix.exs").exists() {
            return;
        }
        if profile.stack.primary_language.is_none() {
            profile.stack.primary_language = Some("elixir".to_string());
        }
        if profile.stack.runtime.is_none() {
            profile.stack.runtime = Some("beam".to_string());
        }
        if profile.stack.package_manager.is_none() {
            profile.stack.package_manager = Some("mix".to_string());
        }
        if profile.stack.framework.is_none() {
            if let Ok(contents) = std::fs::read_to_string(dir.join("mix.exs")) {
                if contents.contains(":phoenix") {
                    profile.stack.framework = Some("phoenix".to_string());
                }
            }
        }
    }

    fn detect_pubspec_yaml(dir: &Path, profile: &mut StackProfile) {
        if !dir.join("pubspec.yaml").exists() {
            return;
        }
        if profile.stack.primary_language.is_none() {
            profile.stack.primary_language = Some("dart".to_string());
        }
        if profile.stack.runtime.is_none() {
            profile.stack.runtime = Some("dart".to_string());
        }
        if profile.stack.package_manager.is_none() {
            profile.stack.package_manager = Some("pub".to_string());
        }
        if profile.stack.framework.is_none() {
            if let Ok(contents) = std::fs::read_to_string(dir.join("pubspec.yaml")) {
                if contents.contains("flutter:") {
                    profile.stack.framework = Some("flutter".to_string());
                    profile.stack.runtime = Some("flutter".to_string());
                }
            }
        }
    }

    fn detect_package_swift(dir: &Path, profile: &mut StackProfile) {
        if !dir.join("Package.swift").exists() {
            return;
        }
        if profile.stack.primary_language.is_none() {
            profile.stack.primary_language = Some("swift".to_string());
        }
        if profile.stack.runtime.is_none() {
            profile.stack.runtime = Some("swift".to_string());
        }
        if profile.stack.package_manager.is_none() {
            profile.stack.package_manager = Some("spm".to_string());
        }
    }

    // ── 3. Lock files ───────────────────────────────────────────────────

    fn detect_lock_files(dir: &Path, profile: &mut StackProfile) {
        // Package manager detection from lock files (only if not already set)
        if profile.stack.package_manager.is_none()
            && (dir.join("package.json").exists() || dir.join("package-lock.json").exists())
        {
            if dir.join("pnpm-lock.yaml").exists() {
                profile.stack.package_manager = Some("pnpm".to_string());
            } else if dir.join("yarn.lock").exists() {
                profile.stack.package_manager = Some("yarn".to_string());
            } else if dir.join("package-lock.json").exists() {
                profile.stack.package_manager = Some("npm".to_string());
            } else if dir.join("bun.lockb").exists() || dir.join("bun.lock").exists() {
                profile.stack.package_manager = Some("bun".to_string());
            }
        }

        if profile.stack.package_manager.is_none() && dir.join("poetry.lock").exists() {
            profile.stack.package_manager = Some("poetry".to_string());
        }

        if profile.stack.package_manager.is_none() && dir.join("Pipfile.lock").exists() {
            profile.stack.package_manager = Some("pipenv".to_string());
        }
    }

    // ── 4. Config files ─────────────────────────────────────────────────

    fn detect_config_files(dir: &Path, profile: &mut StackProfile) {
        // TypeScript
        if dir.join("tsconfig.json").exists() && profile.stack.primary_language.is_none() {
            profile.stack.primary_language = Some("typescript".to_string());
        }

        // Makefile suggests compiled language or complex build
        // (no action needed, just a signal)
    }

    // ── 5. Directory structure ──────────────────────────────────────────

    fn detect_directory_structure(dir: &Path, profile: &mut StackProfile) {
        // React-style project structure
        if profile.stack.framework.is_none() {
            if dir.join("pages").is_dir() && dir.join("package.json").exists() {
                // Could be Next.js or similar; already handled by package.json
            }
            if dir.join("src/app").is_dir() && dir.join("angular.json").exists() {
                profile.stack.framework = Some("angular".to_string());
            }
        }

        // Python project structure
        if profile.stack.primary_language.is_none() && dir.join("manage.py").exists() {
            profile.stack.primary_language = Some("python".to_string());
            profile.stack.runtime = Some("python".to_string());
            if profile.stack.framework.is_none() {
                profile.stack.framework = Some("django".to_string());
            }
        }

        // If we still have no name, use the directory name
        if profile.project.name.is_empty() {
            if let Some(dir_name) = dir.file_name().and_then(|n| n.to_str()) {
                profile.project.name = dir_name.to_string();
            }
        }
    }

    // ── 6. Environment files ────────────────────────────────────────────

    fn detect_env_files(dir: &Path, profile: &mut StackProfile) {
        // Check .env files for service hints
        let env_files = [".env", ".env.local", ".env.development", ".env.example"];
        for env_file in &env_files {
            let path = dir.join(env_file);
            if path.exists() {
                if let Ok(contents) = std::fs::read_to_string(&path) {
                    Self::detect_services_from_env(&contents, profile);
                }
                break; // Only parse the first found env file
            }
        }
    }

    // ── 7. VCS ──────────────────────────────────────────────────────────

    fn detect_vcs(dir: &Path, profile: &mut StackProfile) {
        if profile.project.vcs.is_some() {
            return;
        }
        if dir.join(".git").exists() {
            profile.project.vcs = Some("git".to_string());
        } else if dir.join(".hg").exists() {
            profile.project.vcs = Some("mercurial".to_string());
        } else if dir.join(".svn").exists() {
            profile.project.vcs = Some("svn".to_string());
        }
    }

    // ── 8. Runtime versions ─────────────────────────────────────────────

    fn detect_runtime_versions(dir: &Path, profile: &mut StackProfile) {
        if profile.stack.runtime_version.is_some() {
            return;
        }

        // Node.js version files
        for file in &[".nvmrc", ".node-version"] {
            let path = dir.join(file);
            if path.exists() {
                if let Ok(version) = std::fs::read_to_string(&path) {
                    let version = version.trim().to_string();
                    if !version.is_empty() {
                        profile.stack.runtime_version = Some(version);
                        return;
                    }
                }
            }
        }

        // Python version
        let python_version = dir.join(".python-version");
        if python_version.exists() {
            if let Ok(version) = std::fs::read_to_string(&python_version) {
                let version = version.trim().to_string();
                if !version.is_empty() {
                    profile.stack.runtime_version = Some(version);
                    return;
                }
            }
        }

        // Ruby version
        let ruby_version = dir.join(".ruby-version");
        if ruby_version.exists() {
            if let Ok(version) = std::fs::read_to_string(&ruby_version) {
                let version = version.trim().to_string();
                if !version.is_empty() {
                    profile.stack.runtime_version = Some(version);
                    return;
                }
            }
        }

        // asdf .tool-versions
        let tool_versions = dir.join(".tool-versions");
        if tool_versions.exists() {
            if let Ok(contents) = std::fs::read_to_string(&tool_versions) {
                // Match the runtime to the detected language
                let runtime = profile.stack.runtime.as_deref().unwrap_or_default();
                let search_key = match runtime {
                    "node" | "nodejs" => "nodejs",
                    "python" => "python",
                    "ruby" => "ruby",
                    "go" => "golang",
                    "rust" => "rust",
                    "java" | "jvm" => "java",
                    _ => "",
                };
                if !search_key.is_empty() {
                    for line in contents.lines() {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 && parts[0] == search_key {
                            profile.stack.runtime_version = Some(parts[1].to_string());
                            return;
                        }
                    }
                }
            }
        }

        // Rust toolchain
        let rust_toolchain = dir.join("rust-toolchain.toml");
        if rust_toolchain.exists() {
            if let Ok(contents) = std::fs::read_to_string(&rust_toolchain) {
                if let Ok(tc) = toml::from_str::<toml::Value>(&contents) {
                    if let Some(channel) = tc
                        .get("toolchain")
                        .and_then(|t| t.get("channel"))
                        .and_then(|c| c.as_str())
                    {
                        profile.stack.runtime_version = Some(channel.to_string());
                    }
                }
            }
        }

        // Also try plain rust-toolchain file (no extension)
        let rust_toolchain_plain = dir.join("rust-toolchain");
        if rust_toolchain_plain.exists() {
            if let Ok(version) = std::fs::read_to_string(&rust_toolchain_plain) {
                let version = version.trim().to_string();
                if !version.is_empty() {
                    profile.stack.runtime_version = Some(version);
                }
            }
        }
    }

    // ── 9. CI/CD ────────────────────────────────────────────────────────

    fn detect_ci_cd(dir: &Path, profile: &mut StackProfile) {
        if profile.infrastructure.ci_cd.is_some() {
            return;
        }

        if dir.join(".github/workflows").is_dir() {
            profile.infrastructure.ci_cd = Some("github-actions".to_string());
        } else if dir.join(".gitlab-ci.yml").exists() {
            profile.infrastructure.ci_cd = Some("gitlab-ci".to_string());
        } else if dir.join("Jenkinsfile").exists() {
            profile.infrastructure.ci_cd = Some("jenkins".to_string());
        } else if dir.join(".circleci").is_dir() {
            profile.infrastructure.ci_cd = Some("circleci".to_string());
        } else if dir.join(".travis.yml").exists() {
            profile.infrastructure.ci_cd = Some("travis-ci".to_string());
        } else if dir.join("azure-pipelines.yml").exists() {
            profile.infrastructure.ci_cd = Some("azure-pipelines".to_string());
        } else if dir.join("bitbucket-pipelines.yml").exists() {
            profile.infrastructure.ci_cd = Some("bitbucket-pipelines".to_string());
        } else if dir.join(".buildkite").is_dir() {
            profile.infrastructure.ci_cd = Some("buildkite".to_string());
        }
    }

    // ── 10. Cloud provider ──────────────────────────────────────────────

    fn detect_cloud_provider(dir: &Path, profile: &mut StackProfile) {
        if profile.infrastructure.cloud_provider.is_some() {
            return;
        }

        if dir.join("serverless.yml").exists()
            || dir.join("serverless.yaml").exists()
            || dir.join("samconfig.toml").exists()
            || dir.join("template.yaml").exists()
            || dir.join(".aws").is_dir()
        {
            profile.infrastructure.cloud_provider = Some("aws".to_string());
        } else if dir.join(".gcloud").is_dir()
            || dir.join("app.yaml").exists()
            || dir.join("cloudbuild.yaml").exists()
        {
            profile.infrastructure.cloud_provider = Some("gcp".to_string());
        } else if dir.join("azure-pipelines.yml").exists() {
            profile.infrastructure.cloud_provider = Some("azure".to_string());
        } else if dir.join("fly.toml").exists() {
            profile.infrastructure.cloud_provider = Some("fly".to_string());
        } else if dir.join("render.yaml").exists() {
            profile.infrastructure.cloud_provider = Some("render".to_string());
        } else if dir.join("vercel.json").exists() || dir.join(".vercel").is_dir() {
            profile.infrastructure.cloud_provider = Some("vercel".to_string());
        } else if dir.join("netlify.toml").exists() {
            profile.infrastructure.cloud_provider = Some("netlify".to_string());
        } else if dir.join("railway.toml").exists() || dir.join("railway.json").exists() {
            profile.infrastructure.cloud_provider = Some("railway".to_string());
        }

        // Terraform / IaC
        if dir.join(".terraform").is_dir() || dir.join("main.tf").exists() {
            // Infrastructure as Code detected, but don't override the cloud provider
            // unless nothing else is set.
            if profile.infrastructure.cloud_provider.is_none() {
                profile.infrastructure.cloud_provider = Some("terraform".to_string());
            }
        }

        if (dir.join("pulumi.yaml").exists() || dir.join("Pulumi.yaml").exists())
            && profile.infrastructure.cloud_provider.is_none()
        {
            profile.infrastructure.cloud_provider = Some("pulumi".to_string());
        }
    }

    // ── 11. Docker & orchestration ──────────────────────────────────────

    fn detect_docker(dir: &Path, profile: &mut StackProfile) {
        if dir.join("Dockerfile").exists()
            || dir.join("dockerfile").exists()
            || dir.join(".dockerignore").exists()
        {
            profile.infrastructure.containerized = true;
        }

        if dir.join("docker-compose.yml").exists()
            || dir.join("docker-compose.yaml").exists()
            || dir.join("compose.yml").exists()
            || dir.join("compose.yaml").exists()
        {
            profile.infrastructure.containerized = true;
            if profile.infrastructure.orchestration.is_none() {
                profile.infrastructure.orchestration = Some("docker-compose".to_string());
            }
        }

        if (dir.join("k8s").is_dir()
            || dir.join("kubernetes").is_dir()
            || dir.join("helm").is_dir()
            || dir.join("Chart.yaml").exists())
            && profile.infrastructure.orchestration.is_none()
        {
            profile.infrastructure.orchestration = Some("kubernetes".to_string());
        }

        if dir.join("skaffold.yaml").exists() && profile.infrastructure.orchestration.is_none() {
            profile.infrastructure.orchestration = Some("skaffold".to_string());
        }
    }

    // ── 12. Services (database, cache, message queue) ───────────────────

    fn detect_services(dir: &Path, profile: &mut StackProfile) {
        // Try to detect from docker-compose
        let compose_files = [
            "docker-compose.yml",
            "docker-compose.yaml",
            "compose.yml",
            "compose.yaml",
        ];
        for compose_file in &compose_files {
            let path = dir.join(compose_file);
            if path.exists() {
                if let Ok(contents) = std::fs::read_to_string(&path) {
                    Self::detect_services_from_compose(&contents, profile);
                }
                break;
            }
        }
    }

    fn detect_services_from_compose(contents: &str, profile: &mut StackProfile) {
        // Simple string-based detection from docker-compose contents.
        // We avoid pulling in a YAML parser to keep dependencies light;
        // image names in compose files are distinctive enough.
        let lower = contents.to_lowercase();

        if profile.services.database.is_none() {
            if lower.contains("postgres") || lower.contains("postgresql") {
                profile.services.database = Some("postgresql".to_string());
            } else if lower.contains("mysql") || lower.contains("mariadb") {
                profile.services.database = Some("mysql".to_string());
            } else if lower.contains("mongo") {
                profile.services.database = Some("mongodb".to_string());
            } else if lower.contains("cockroach") {
                profile.services.database = Some("cockroachdb".to_string());
            } else if lower.contains("sqlite") {
                profile.services.database = Some("sqlite".to_string());
            }
        }

        if profile.services.cache.is_none() {
            if lower.contains("redis") {
                profile.services.cache = Some("redis".to_string());
            } else if lower.contains("memcached") || lower.contains("memcache") {
                profile.services.cache = Some("memcached".to_string());
            } else if lower.contains("dragonfly") {
                profile.services.cache = Some("dragonfly".to_string());
            }
        }

        if profile.services.message_queue.is_none() {
            if lower.contains("rabbitmq") {
                profile.services.message_queue = Some("rabbitmq".to_string());
            } else if lower.contains("kafka") {
                profile.services.message_queue = Some("kafka".to_string());
            } else if lower.contains("nats") {
                profile.services.message_queue = Some("nats".to_string());
            } else if lower.contains("pulsar") {
                profile.services.message_queue = Some("pulsar".to_string());
            }
        }
    }

    fn detect_services_from_env(contents: &str, profile: &mut StackProfile) {
        let lower = contents.to_lowercase();

        if profile.services.database.is_none()
            && (lower.contains("database_url") || lower.contains("db_url"))
        {
            if lower.contains("postgres") || lower.contains("postgresql") {
                profile.services.database = Some("postgresql".to_string());
            } else if lower.contains("mysql") {
                profile.services.database = Some("mysql".to_string());
            } else if lower.contains("mongo") {
                profile.services.database = Some("mongodb".to_string());
            } else if lower.contains("sqlite") {
                profile.services.database = Some("sqlite".to_string());
            }
        }

        if profile.services.cache.is_none()
            && (lower.contains("redis_url") || lower.contains("redis_host"))
        {
            profile.services.cache = Some("redis".to_string());
        } else if profile.services.cache.is_none() && lower.contains("memcached") {
            profile.services.cache = Some("memcached".to_string());
        }

        if profile.services.message_queue.is_none()
            && (lower.contains("rabbitmq") || lower.contains("amqp_url"))
        {
            profile.services.message_queue = Some("rabbitmq".to_string());
        } else if profile.services.message_queue.is_none() && lower.contains("kafka") {
            profile.services.message_queue = Some("kafka".to_string());
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
    fn test_detect_empty_directory() {
        let dir = setup_dir();
        let profile = ContextDetector::detect(dir.path()).unwrap();
        // Should succeed and use directory name as project name
        assert!(!profile.project.name.is_empty());
        assert_eq!(profile.project.vcs, None);
    }

    #[test]
    fn test_detect_node_project() {
        let dir = setup_dir();
        let package_json = r#"{
            "name": "my-app",
            "scripts": {
                "dev": "next dev",
                "build": "next build",
                "test": "jest"
            },
            "dependencies": {
                "next": "15.2.0",
                "react": "19.0.0"
            },
            "devDependencies": {
                "typescript": "5.0.0",
                "jest": "29.0.0",
                "eslint": "8.0.0",
                "prettier": "3.0.0"
            }
        }"#;
        std::fs::write(dir.path().join("package.json"), package_json).unwrap();
        std::fs::write(dir.path().join("package-lock.json"), "{}").unwrap();
        std::fs::write(dir.path().join("tsconfig.json"), "{}").unwrap();

        let profile = ContextDetector::detect(dir.path()).unwrap();
        assert_eq!(profile.project.name, "my-app");
        assert_eq!(
            profile.stack.primary_language.as_deref(),
            Some("typescript")
        );
        assert_eq!(profile.stack.runtime.as_deref(), Some("node"));
        assert_eq!(profile.stack.framework.as_deref(), Some("next"));
        assert_eq!(profile.stack.framework_version.as_deref(), Some("15.2.0"));
        assert_eq!(profile.stack.package_manager.as_deref(), Some("npm"));
        assert_eq!(profile.stack.test_runner.as_deref(), Some("jest"));
        assert_eq!(profile.stack.linter.as_deref(), Some("eslint"));
        assert_eq!(profile.stack.formatter.as_deref(), Some("prettier"));
        assert_eq!(
            profile.scripts.get("dev").map(String::as_str),
            Some("next dev")
        );
    }

    #[test]
    fn test_detect_rust_project() {
        let dir = setup_dir();
        let cargo_toml = r#"
[package]
name = "my-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
axum = "0.7"
"#;
        std::fs::write(dir.path().join("Cargo.toml"), cargo_toml).unwrap();

        let profile = ContextDetector::detect(dir.path()).unwrap();
        assert_eq!(profile.project.name, "my-crate");
        assert_eq!(profile.stack.primary_language.as_deref(), Some("rust"));
        assert_eq!(profile.stack.framework.as_deref(), Some("axum"));
        assert_eq!(profile.stack.package_manager.as_deref(), Some("cargo"));
    }

    #[test]
    fn test_detect_python_project() {
        let dir = setup_dir();
        let pyproject = r#"
[project]
name = "my-api"
dependencies = [
    "fastapi>=0.100.0",
    "uvicorn",
]

[tool.pytest.ini_options]
testpaths = ["tests"]

[tool.ruff]
line-length = 88

[tool.black]
line-length = 88
"#;
        std::fs::write(dir.path().join("pyproject.toml"), pyproject).unwrap();

        let profile = ContextDetector::detect(dir.path()).unwrap();
        assert_eq!(profile.project.name, "my-api");
        assert_eq!(profile.stack.primary_language.as_deref(), Some("python"));
        assert_eq!(profile.stack.framework.as_deref(), Some("fastapi"));
        assert_eq!(profile.stack.test_runner.as_deref(), Some("pytest"));
        assert_eq!(profile.stack.linter.as_deref(), Some("ruff"));
        assert_eq!(profile.stack.formatter.as_deref(), Some("black"));
    }

    #[test]
    fn test_detect_go_project() {
        let dir = setup_dir();
        let go_mod = "module github.com/example/my-service\n\ngo 1.21\n";
        std::fs::write(dir.path().join("go.mod"), go_mod).unwrap();

        let profile = ContextDetector::detect(dir.path()).unwrap();
        assert_eq!(profile.project.name, "my-service");
        assert_eq!(profile.stack.primary_language.as_deref(), Some("go"));
        assert_eq!(profile.stack.runtime_version.as_deref(), Some("1.21"));
        assert_eq!(profile.stack.package_manager.as_deref(), Some("go modules"));
    }

    #[test]
    fn test_detect_git_repo() {
        let dir = setup_dir();
        std::fs::create_dir(dir.path().join(".git")).unwrap();

        let profile = ContextDetector::detect(dir.path()).unwrap();
        assert_eq!(profile.project.vcs.as_deref(), Some("git"));
    }

    #[test]
    fn test_detect_ci_cd() {
        let dir = setup_dir();
        std::fs::create_dir_all(dir.path().join(".github/workflows")).unwrap();

        let profile = ContextDetector::detect(dir.path()).unwrap();
        assert_eq!(
            profile.infrastructure.ci_cd.as_deref(),
            Some("github-actions")
        );
    }

    #[test]
    fn test_detect_docker() {
        let dir = setup_dir();
        std::fs::write(dir.path().join("Dockerfile"), "FROM node:22").unwrap();
        std::fs::write(
            dir.path().join("docker-compose.yml"),
            "services:\n  db:\n    image: postgres:16\n  redis:\n    image: redis:7\n  mq:\n    image: rabbitmq:3\n",
        )
        .unwrap();

        let profile = ContextDetector::detect(dir.path()).unwrap();
        assert!(profile.infrastructure.containerized);
        assert_eq!(
            profile.infrastructure.orchestration.as_deref(),
            Some("docker-compose")
        );
        assert_eq!(profile.services.database.as_deref(), Some("postgresql"));
        assert_eq!(profile.services.cache.as_deref(), Some("redis"));
        assert_eq!(profile.services.message_queue.as_deref(), Some("rabbitmq"));
    }

    #[test]
    fn test_detect_nvmrc() {
        let dir = setup_dir();
        let package_json = r#"{"name":"app","dependencies":{}}"#;
        std::fs::write(dir.path().join("package.json"), package_json).unwrap();
        std::fs::write(dir.path().join(".nvmrc"), "v20.10.0\n").unwrap();

        let profile = ContextDetector::detect(dir.path()).unwrap();
        assert_eq!(profile.stack.runtime_version.as_deref(), Some("v20.10.0"));
    }

    #[test]
    fn test_detect_cloud_provider_vercel() {
        let dir = setup_dir();
        std::fs::write(dir.path().join("vercel.json"), "{}").unwrap();

        let profile = ContextDetector::detect(dir.path()).unwrap();
        assert_eq!(
            profile.infrastructure.cloud_provider.as_deref(),
            Some("vercel")
        );
    }

    #[test]
    fn test_detect_services_from_env() {
        let dir = setup_dir();
        let env = "DATABASE_URL=postgresql://localhost/mydb\nREDIS_URL=redis://localhost:6379\n";
        std::fs::write(dir.path().join(".env"), env).unwrap();

        let profile = ContextDetector::detect(dir.path()).unwrap();
        assert_eq!(profile.services.database.as_deref(), Some("postgresql"));
        assert_eq!(profile.services.cache.as_deref(), Some("redis"));
    }

    #[test]
    fn test_deftshell_config_overrides() {
        let dir = setup_dir();
        // Create a package.json that would set framework to react
        let package_json = r#"{"name":"app","dependencies":{"react":"18.0.0"}}"#;
        std::fs::write(dir.path().join("package.json"), package_json).unwrap();

        // .deftshell.toml overrides framework
        let deftshell = r#"
[project]
name = "custom-name"

[stack]
framework = "custom-framework"
"#;
        std::fs::write(dir.path().join(".deftshell.toml"), deftshell).unwrap();

        let profile = ContextDetector::detect(dir.path()).unwrap();
        assert_eq!(profile.project.name, "custom-name");
        assert_eq!(profile.stack.framework.as_deref(), Some("custom-framework"));
    }

    #[test]
    fn test_detect_yarn_package_manager() {
        let dir = setup_dir();
        let package_json = r#"{"name":"app","dependencies":{}}"#;
        std::fs::write(dir.path().join("package.json"), package_json).unwrap();
        std::fs::write(dir.path().join("yarn.lock"), "").unwrap();

        let profile = ContextDetector::detect(dir.path()).unwrap();
        assert_eq!(profile.stack.package_manager.as_deref(), Some("yarn"));
    }

    #[test]
    fn test_detect_pnpm_package_manager() {
        let dir = setup_dir();
        let package_json = r#"{"name":"app","dependencies":{}}"#;
        std::fs::write(dir.path().join("package.json"), package_json).unwrap();
        std::fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();

        let profile = ContextDetector::detect(dir.path()).unwrap();
        assert_eq!(profile.stack.package_manager.as_deref(), Some("pnpm"));
    }
}
