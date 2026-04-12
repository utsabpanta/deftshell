use crate::storage::db::Database;
use anyhow::Result;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

#[derive(Debug, Clone)]
pub struct Suggestion {
    pub kind: SuggestionKind,
    pub message: String,
    pub action: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SuggestionKind {
    Typo,
    Alias,
    Sequence,
    Reminder,
    Frequency,
}

pub struct SuggestionEngine<'a> {
    db: &'a Database,
    matcher: SkimMatcherV2,
}

impl<'a> SuggestionEngine<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self {
            db,
            matcher: SkimMatcherV2::default(),
        }
    }

    /// Check if a command looks like a typo of a common command
    pub fn check_typo(&self, command: &str) -> Option<Suggestion> {
        let first_word = command.split_whitespace().next()?;

        let common_commands = [
            "git",
            "docker",
            "npm",
            "yarn",
            "pnpm",
            "cargo",
            "python",
            "pip",
            "node",
            "npx",
            "kubectl",
            "terraform",
            "make",
            "curl",
            "wget",
            "ssh",
            "scp",
            "rsync",
            "find",
            "grep",
            "awk",
            "sed",
            "cat",
            "ls",
            "cd",
            "mkdir",
            "rm",
            "cp",
            "mv",
        ];

        // Check for exact match first — not a typo
        if common_commands.contains(&first_word) {
            return None;
        }

        // Find the closest command by edit distance
        let mut best: Option<(&str, usize)> = None;
        for cmd in &common_commands {
            let distance = Self::edit_distance(first_word, cmd);
            if distance > 0 && distance <= 2 && (best.is_none() || distance < best.unwrap().1) {
                best = Some((cmd, distance));
            }
        }

        // Also consider fuzzy matching for longer commands where edit distance
        // may miss partial matches (e.g., "kubeclt" vs "kubectl").
        if best.is_none() {
            for cmd in &common_commands {
                if let Some(score) = self.matcher.fuzzy_match(cmd, first_word) {
                    if score > 20 {
                        let distance = Self::edit_distance(first_word, cmd);
                        if distance > 0 && distance <= 3 {
                            best = Some((cmd, distance));
                            break;
                        }
                    }
                }
            }
        }

        if let Some((matched, _)) = best {
            let corrected = command.replacen(first_word, matched, 1);
            return Some(Suggestion {
                kind: SuggestionKind::Typo,
                message: format!("Did you mean `{}`?", corrected),
                action: Some(corrected),
            });
        }

        None
    }

    /// Suggest creating an alias for frequently repeated commands
    pub fn check_alias_suggestion(&self, directory: &str) -> Result<Option<Suggestion>> {
        let stats = self.db.get_command_stats(directory)?;
        for (cmd, count) in &stats.most_used {
            if *count >= 20 && cmd.len() > 15 {
                let alias_name = Self::generate_alias_name(cmd);
                return Ok(Some(Suggestion {
                    kind: SuggestionKind::Alias,
                    message: format!(
                        "You've run `{}` {} times. Create alias `{}`?",
                        cmd, count, alias_name
                    ),
                    action: Some(format!("ds alias add {}=\"{}\"", alias_name, cmd)),
                }));
            }
        }
        Ok(None)
    }

    /// Detect command sequences that are often run together
    pub fn check_sequence(
        &self,
        last_command: &str,
        directory: &str,
    ) -> Result<Option<Suggestion>> {
        let recent = self.db.get_recent_commands(directory, 100)?;
        let mut sequence_counts: std::collections::HashMap<String, u32> =
            std::collections::HashMap::new();

        // Find commands that commonly follow the last command
        for window in recent.windows(2) {
            if window[0].command == last_command {
                *sequence_counts
                    .entry(window[1].command.clone())
                    .or_insert(0) += 1;
            }
        }

        // Find the most common follower
        if let Some((next_cmd, count)) = sequence_counts.iter().max_by_key(|(_, v)| *v) {
            if *count >= 5 {
                return Ok(Some(Suggestion {
                    kind: SuggestionKind::Sequence,
                    message: format!("You usually run `{}` after this. Run it now?", next_cmd),
                    action: Some(next_cmd.clone()),
                }));
            }
        }

        Ok(None)
    }

    fn generate_alias_name(command: &str) -> String {
        let words: Vec<&str> = command.split_whitespace().collect();
        if words.len() <= 1 {
            return command.chars().take(3).collect();
        }
        // Take first letter of each word
        words
            .iter()
            .take(4)
            .map(|w| w.chars().next().unwrap_or('x'))
            .collect()
    }

    fn edit_distance(a: &str, b: &str) -> usize {
        let a: Vec<char> = a.chars().collect();
        let b: Vec<char> = b.chars().collect();
        let mut dp = vec![vec![0usize; b.len() + 1]; a.len() + 1];

        for (i, row) in dp.iter_mut().enumerate().take(a.len() + 1) {
            row[0] = i;
        }
        for (j, val) in dp[0].iter_mut().enumerate().take(b.len() + 1) {
            *val = j;
        }

        for i in 1..=a.len() {
            for j in 1..=b.len() {
                let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
                dp[i][j] = (dp[i - 1][j] + 1)
                    .min(dp[i][j - 1] + 1)
                    .min(dp[i - 1][j - 1] + cost);
            }
        }

        dp[a.len()][b.len()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edit_distance() {
        // Levenshtein: transposition "git" <-> "gti" = 2 (replace+replace)
        assert_eq!(SuggestionEngine::edit_distance("git", "gti"), 2);
        // Deletion: "docker" -> "docer" = 1
        assert_eq!(SuggestionEngine::edit_distance("docker", "docer"), 1);
        assert_eq!(SuggestionEngine::edit_distance("abc", "abc"), 0);
    }

    #[test]
    fn test_generate_alias_name() {
        assert_eq!(SuggestionEngine::generate_alias_name("npm run test"), "nrt");
        // First char of each word: "docker-compose" -> 'd', "up" -> 'u', "-d" -> '-', "--build" -> '-'
        assert_eq!(
            SuggestionEngine::generate_alias_name("docker-compose up -d --build"),
            "du--"
        );
    }
}
