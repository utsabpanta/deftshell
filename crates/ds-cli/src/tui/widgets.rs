use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// Render a status card widget
pub fn status_card(frame: &mut Frame, area: Rect, title: &str, value: &str, color: Color) {
    let card = Paragraph::new(vec![Line::from(Span::styled(
        value,
        Style::default().fg(color),
    ))])
    .block(Block::default().borders(Borders::ALL).title(title))
    .wrap(Wrap { trim: true });
    frame.render_widget(card, area);
}

/// Format a sparkline from a series of values
pub fn sparkline_string(values: &[u64]) -> String {
    let blocks = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    if values.is_empty() {
        return String::new();
    }
    let max = *values.iter().max().unwrap_or(&1) as f64;
    if max == 0.0 {
        return "▁".repeat(values.len());
    }
    values
        .iter()
        .map(|&v| {
            let idx = ((v as f64 / max) * 7.0) as usize;
            blocks[idx.min(7)]
        })
        .collect()
}

/// Format bytes into human-readable string
pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

/// Format a duration in a human-friendly way
pub fn format_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60_000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else if ms < 3_600_000 {
        let mins = ms / 60_000;
        let secs = (ms % 60_000) / 1000;
        format!("{}m {}s", mins, secs)
    } else {
        let hours = ms / 3_600_000;
        let mins = (ms % 3_600_000) / 60_000;
        format!("{}h {}m", hours, mins)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparkline() {
        let values = vec![1, 4, 2, 8, 5, 3, 7, 6];
        let spark = sparkline_string(&values);
        assert_eq!(spark.chars().count(), 8);
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1_048_576), "1.0 MB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(500), "500ms");
        assert_eq!(format_duration(1500), "1.5s");
        assert_eq!(format_duration(90_000), "1m 30s");
        assert_eq!(format_duration(3_660_000), "1h 1m");
    }
}
