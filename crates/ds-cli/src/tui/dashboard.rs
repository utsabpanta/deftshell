use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ds_core::intelligence::analytics::AnalyticsSummary;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::CrosstermBackend,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Bar, BarChart, BarGroup, Block, Borders, Cell, Paragraph, Row, Table, Wrap},
    Frame, Terminal,
};
use std::io;

pub struct Dashboard {
    summary: AnalyticsSummary,
}

impl Dashboard {
    pub fn new(summary: AnalyticsSummary) -> Self {
        Self { summary }
    }

    pub fn run(&self) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let result = self.run_loop(&mut terminal);

        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result
    }

    fn run_loop(&self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        loop {
            terminal.draw(|f| self.render(f))?;

            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                        _ => {}
                    }
                }
            }
        }
    }

    fn render(&self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Length(8), // Summary cards
                Constraint::Min(10),   // Charts
                Constraint::Length(3), // Footer
            ])
            .split(frame.area());

        self.render_title(frame, chunks[0]);
        self.render_summary(frame, chunks[1]);
        self.render_charts(frame, chunks[2]);
        self.render_footer(frame, chunks[3]);
    }

    fn render_title(&self, frame: &mut Frame, area: Rect) {
        let title = Paragraph::new(Line::from(vec![
            Span::styled(
                " DeftShell Analytics ",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" — {} ", self.summary.period),
                Style::default().fg(Color::Gray),
            ),
        ]))
        .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, area);
    }

    fn render_summary(&self, frame: &mut Frame, area: Rect) {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ])
            .split(area);

        // Total commands
        let total = Paragraph::new(vec![
            Line::from(Span::styled(
                format!("{}", self.summary.total_commands),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "Total Commands",
                Style::default().fg(Color::Gray),
            )),
        ])
        .block(Block::default().borders(Borders::ALL).title("Commands"));
        frame.render_widget(total, cols[0]);

        // Unique commands
        let unique = Paragraph::new(vec![
            Line::from(Span::styled(
                format!("{}", self.summary.unique_commands),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "Unique Commands",
                Style::default().fg(Color::Gray),
            )),
        ])
        .block(Block::default().borders(Borders::ALL).title("Unique"));
        frame.render_widget(unique, cols[1]);

        // Error rate
        let error_color = if self.summary.error_rate > 10.0 {
            Color::Red
        } else if self.summary.error_rate > 5.0 {
            Color::Yellow
        } else {
            Color::Green
        };
        let errors = Paragraph::new(vec![
            Line::from(Span::styled(
                format!("{:.1}%", self.summary.error_rate),
                Style::default()
                    .fg(error_color)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled("Error Rate", Style::default().fg(Color::Gray))),
        ])
        .block(Block::default().borders(Borders::ALL).title("Errors"));
        frame.render_widget(errors, cols[2]);

        // AI usage
        let ai = Paragraph::new(vec![
            Line::from(Span::styled(
                format!(
                    "{}",
                    self.summary.ai_usage.total_tokens_in + self.summary.ai_usage.total_tokens_out
                ),
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!("AI Tokens (~${:.2})", self.summary.ai_usage.estimated_cost),
                Style::default().fg(Color::Gray),
            )),
        ])
        .block(Block::default().borders(Borders::ALL).title("AI Usage"));
        frame.render_widget(ai, cols[3]);
    }

    fn render_charts(&self, frame: &mut Frame, area: Rect) {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Most used commands bar chart
        let bars: Vec<Bar> = self
            .summary
            .most_used
            .iter()
            .take(8)
            .map(|(cmd, count)| {
                let label = if cmd.len() > 20 {
                    format!("{}...", &cmd[..17])
                } else {
                    cmd.clone()
                };
                Bar::default()
                    .value(*count)
                    .label(Line::from(label))
                    .style(Style::default().fg(Color::Cyan))
            })
            .collect();

        let chart = BarChart::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Most Used Commands"),
            )
            .data(BarGroup::default().bars(&bars))
            .bar_width(3)
            .bar_gap(1)
            .direction(Direction::Horizontal);
        frame.render_widget(chart, cols[0]);

        // Right side: detailed stats table
        let rows: Vec<Row> = self
            .summary
            .most_used
            .iter()
            .take(15)
            .enumerate()
            .map(|(i, (cmd, count))| {
                Row::new(vec![
                    Cell::from(format!("{}", i + 1)).style(Style::default().fg(Color::Gray)),
                    Cell::from(cmd.as_str()).style(Style::default().fg(Color::White)),
                    Cell::from(format!("{}", count)).style(Style::default().fg(Color::Cyan)),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(4),
                Constraint::Min(20),
                Constraint::Length(8),
            ],
        )
        .header(
            Row::new(vec!["#", "Command", "Count"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Command Frequency"),
        );
        frame.render_widget(table, cols[1]);
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let footer = Paragraph::new(Line::from(vec![
            Span::styled(
                " q ",
                Style::default()
                    .fg(Color::White)
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Quit  ", Style::default().fg(Color::Gray)),
            Span::styled(" DeftShell v", Style::default().fg(Color::Gray)),
            Span::styled(env!("CARGO_PKG_VERSION"), Style::default().fg(Color::Gray)),
        ]))
        .wrap(Wrap { trim: true });
        frame.render_widget(footer, area);
    }
}
