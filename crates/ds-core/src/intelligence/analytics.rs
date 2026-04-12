use crate::storage::db::{Database, UsagePeriod};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsSummary {
    pub period: String,
    pub total_commands: u64,
    pub unique_commands: u64,
    pub error_rate: f64,
    pub most_used: Vec<(String, u64)>,
    pub commands_by_hour: Vec<u64>,
    pub project_time: HashMap<String, u64>,
    pub ai_usage: AiUsageSummary,
    pub safety_stats: SafetyStats,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AiUsageSummary {
    pub total_queries: u64,
    pub total_tokens_in: u64,
    pub total_tokens_out: u64,
    pub estimated_cost: f64,
    pub by_provider: HashMap<String, ProviderUsage>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderUsage {
    pub queries: u64,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub estimated_cost: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SafetyStats {
    pub total_intercepted: u64,
    pub by_level: HashMap<String, u64>,
}

pub struct AnalyticsEngine<'a> {
    db: &'a Database,
}

impl<'a> AnalyticsEngine<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Get analytics for today
    pub fn today(&self) -> Result<AnalyticsSummary> {
        self.for_period("today")
    }

    /// Get analytics for this week
    pub fn this_week(&self) -> Result<AnalyticsSummary> {
        self.for_period("week")
    }

    /// Get analytics for current project
    pub fn for_project(&self, project_dir: &str) -> Result<AnalyticsSummary> {
        let stats = self.db.get_command_stats(project_dir)?;
        let ai_usage = self.db.get_ai_usage(UsagePeriod::All)?;

        Ok(AnalyticsSummary {
            period: format!("project:{}", project_dir),
            total_commands: stats.total_commands,
            unique_commands: stats.unique_commands,
            error_rate: stats.error_rate,
            most_used: stats.most_used,
            commands_by_hour: vec![0; 24],
            project_time: HashMap::new(),
            ai_usage: AiUsageSummary {
                total_tokens_in: ai_usage.total_tokens_in,
                total_tokens_out: ai_usage.total_tokens_out,
                total_queries: 0,
                estimated_cost: ai_usage.total_cost,
                by_provider: HashMap::new(),
            },
            safety_stats: SafetyStats::default(),
        })
    }

    fn for_period(&self, period: &str) -> Result<AnalyticsSummary> {
        let stats = self.db.get_command_stats("*")?;
        let usage_period = match period {
            "today" => UsagePeriod::Today,
            "week" => UsagePeriod::Week,
            "month" => UsagePeriod::Month,
            _ => UsagePeriod::All,
        };
        let ai_usage = self.db.get_ai_usage(usage_period)?;

        Ok(AnalyticsSummary {
            period: period.to_string(),
            total_commands: stats.total_commands,
            unique_commands: stats.unique_commands,
            error_rate: stats.error_rate,
            most_used: stats.most_used,
            commands_by_hour: vec![0; 24],
            project_time: HashMap::new(),
            ai_usage: AiUsageSummary {
                total_tokens_in: ai_usage.total_tokens_in,
                total_tokens_out: ai_usage.total_tokens_out,
                total_queries: 0,
                estimated_cost: ai_usage.total_cost,
                by_provider: HashMap::new(),
            },
            safety_stats: SafetyStats::default(),
        })
    }

    /// Export analytics as JSON
    pub fn export_json(&self, period: &str) -> Result<String> {
        let summary = self.for_period(period)?;
        Ok(serde_json::to_string_pretty(&summary)?)
    }

    /// Export analytics as CSV
    pub fn export_csv(&self, period: &str) -> Result<String> {
        let summary = self.for_period(period)?;
        let mut csv = String::from("command,count\n");
        for (cmd, count) in &summary.most_used {
            csv.push_str(&format!("\"{}\",{}\n", cmd.replace('"', "\"\""), count));
        }
        Ok(csv)
    }
}
