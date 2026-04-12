use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The full detected profile for a project directory, capturing the technology
/// stack, infrastructure, external services, and available scripts.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StackProfile {
    pub project: ProjectInfo,
    pub stack: StackInfo,
    pub infrastructure: InfrastructureInfo,
    pub services: ServicesInfo,
    pub scripts: HashMap<String, String>,
}

/// Basic project-level metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub name: String,
    pub root: String,
    pub vcs: Option<String>,
}

/// Detected technology stack: language, runtime, frameworks, and tooling.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StackInfo {
    pub primary_language: Option<String>,
    pub runtime: Option<String>,
    pub runtime_version: Option<String>,
    pub framework: Option<String>,
    pub framework_version: Option<String>,
    pub package_manager: Option<String>,
    pub test_runner: Option<String>,
    pub linter: Option<String>,
    pub formatter: Option<String>,
    pub bundler: Option<String>,
}

/// Infrastructure signals: containers, orchestration, CI/CD, cloud.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InfrastructureInfo {
    pub containerized: bool,
    pub orchestration: Option<String>,
    pub cloud_provider: Option<String>,
    pub ci_cd: Option<String>,
}

/// External services detected from compose files, env vars, etc.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServicesInfo {
    pub database: Option<String>,
    pub cache: Option<String>,
    pub message_queue: Option<String>,
}
