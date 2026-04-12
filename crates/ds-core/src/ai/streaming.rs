use anyhow::Result;
use colored::Colorize;
use futures::StreamExt;
use std::io::{self, Write};
use std::pin::Pin;

use super::gateway::StreamChunk;

/// Result of printing a streamed AI response.
pub struct StreamResult {
    /// Total number of characters printed.
    pub chars_out: usize,
}

impl StreamResult {
    /// Rough estimate of output tokens (~4 chars per token).
    pub fn estimated_tokens(&self) -> u64 {
        (self.chars_out / 4).max(1) as u64
    }
}

/// Prints a streamed AI response to the terminal with clean formatting.
///
/// Code blocks are visually distinguished from prose using the terminal's
/// own colors — no 24-bit theme injection that clashes with user terminals.
pub struct StreamPrinter;

impl StreamPrinter {
    pub fn new() -> Self {
        Self
    }

    /// Consume the stream, printing each chunk to stdout.
    ///
    /// Code blocks delimited by triple-backtick fences are printed with a
    /// subtle indent and the terminal's default foreground.  Prose is printed
    /// as-is.
    ///
    /// Returns a [`StreamResult`] with statistics about the printed content.
    pub async fn print_stream(
        &self,
        mut stream: Pin<Box<dyn futures::Stream<Item = Result<StreamChunk>> + Send>>,
    ) -> Result<StreamResult> {
        let mut stdout = io::stdout();

        // Accumulation buffer -- we process complete lines.
        let mut buffer = String::new();
        let mut total_chars: usize = 0;

        // State tracking for fenced code blocks.
        let mut in_code_block = false;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;

            if chunk.done {
                break;
            }

            total_chars += chunk.content.len();
            buffer.push_str(&chunk.content);

            // Process all complete lines in the buffer.
            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..=newline_pos].to_string();
                buffer = buffer[newline_pos + 1..].to_string();

                let trimmed = line.trim();

                // Detect code-fence boundaries.
                if trimmed.starts_with("```") {
                    if in_code_block {
                        // Closing fence.
                        writeln!(stdout, "{}", "```".dimmed())?;
                        in_code_block = false;
                    } else {
                        in_code_block = true;
                        // Print the full fence line (with language tag) dimmed.
                        write!(stdout, "{}", trimmed.dimmed())?;
                        writeln!(stdout)?;
                    }
                    continue;
                }

                if in_code_block {
                    // Code: print with indent, default terminal foreground.
                    // The terminal's own color scheme handles readability.
                    write!(stdout, "  {}", line)?;
                } else {
                    // Prose: print as-is in default terminal color.
                    write!(stdout, "{}", line)?;
                }
            }

            stdout.flush()?;
        }

        // Flush anything remaining.
        if in_code_block {
            writeln!(stdout, "{}", "```".dimmed())?;
        }

        if !buffer.is_empty() {
            write!(stdout, "{}", buffer)?;
        }

        writeln!(stdout)?;
        stdout.flush()?;

        Ok(StreamResult {
            chars_out: total_chars,
        })
    }
}

impl Default for StreamPrinter {
    fn default() -> Self {
        Self::new()
    }
}
