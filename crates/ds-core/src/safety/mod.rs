//! Safety engine for DeftShell.
//!
//! This module provides command interception, risk assessment, and safety rules
//! to protect users from accidentally running dangerous commands.
//!
//! # Architecture
//!
//! - **rules**: Defines `SafetyRule`, `RiskLevel`, and the `BuiltinRules` catalog of
//!   known dangerous command patterns.
//! - **interceptor**: The `CommandInterceptor` checks commands against rules,
//!   allowlists, and denylists, producing `SafetyAlert`s.
//! - **assessor**: The `RiskAssessor` performs context-aware risk elevation based on
//!   the current environment (branch, production, Kubernetes context).

pub mod assessor;
pub mod interceptor;
pub mod rules;

// Re-export key types for convenient access.
pub use assessor::{AssessedRisk, RiskAssessor};
pub use interceptor::{CommandInterceptor, InterceptionContext, SafetyAlert};
pub use rules::{BuiltinRules, RiskLevel, SafetyRule};
