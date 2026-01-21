//! Log entry types and buffer management

use std::collections::VecDeque;
use chrono::{DateTime, Utc};

/// Log severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn symbol(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRC",
            LogLevel::Debug => "DBG",
            LogLevel::Info => "INF",
            LogLevel::Warn => "WRN",
            LogLevel::Error => "ERR",
        }
    }

    pub fn color(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            LogLevel::Trace => Color::DarkGray,
            LogLevel::Debug => Color::Gray,
            LogLevel::Info => Color::Green,
            LogLevel::Warn => Color::Yellow,
            LogLevel::Error => Color::Red,
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.symbol())
    }
}

/// A single log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub source: String,
    pub message: String,
}

impl LogEntry {
    pub fn new(level: LogLevel, source: &str, message: &str) -> Self {
        Self {
            timestamp: Utc::now(),
            level,
            source: source.to_string(),
            message: message.to_string(),
        }
    }

    pub fn trace(message: &str) -> Self {
        Self::new(LogLevel::Trace, "tui", message)
    }

    pub fn debug(message: &str) -> Self {
        Self::new(LogLevel::Debug, "tui", message)
    }

    pub fn info(message: &str) -> Self {
        Self::new(LogLevel::Info, "tui", message)
    }

    pub fn warn(message: &str) -> Self {
        Self::new(LogLevel::Warn, "tui", message)
    }

    pub fn error(message: &str) -> Self {
        Self::new(LogLevel::Error, "tui", message)
    }

    pub fn from_source(level: LogLevel, source: &str, message: &str) -> Self {
        Self::new(level, source, message)
    }

    /// Format as terminal-style log line
    pub fn format(&self) -> String {
        format!(
            "{} {} [{}] {}",
            self.timestamp.format("%H:%M:%S%.3f"),
            self.level.symbol(),
            self.source,
            self.message
        )
    }
}

/// Circular buffer for log entries
#[derive(Debug)]
pub struct LogBuffer {
    entries: VecDeque<LogEntry>,
    capacity: usize,
}

impl LogBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, entry: LogEntry) {
        if self.entries.len() >= self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get entries filtered by level
    pub fn filtered(&self, min_level: LogLevel) -> Vec<&LogEntry> {
        self.entries
            .iter()
            .filter(|e| e.level >= min_level)
            .collect()
    }

    /// Get last N entries
    pub fn tail(&self, n: usize) -> Vec<&LogEntry> {
        self.entries.iter().rev().take(n).rev().collect()
    }

    /// Get entries with scroll offset
    pub fn scroll(&self, offset: usize, count: usize, min_level: LogLevel) -> Vec<&LogEntry> {
        let filtered: Vec<_> = self.filtered(min_level);
        let len = filtered.len();
        
        if len == 0 || offset >= len {
            return vec![];
        }

        let start = len.saturating_sub(offset + count);
        let end = len.saturating_sub(offset);
        
        filtered[start..end].to_vec()
    }

    /// Iterate all entries
    pub fn iter(&self) -> impl Iterator<Item = &LogEntry> {
        self.entries.iter()
    }
}

/// Strip ANSI escape codes from a string
fn strip_ansi_codes(s: &str) -> String {
    // Match ANSI escape sequences: ESC[...m for colors/styles
    // Pattern: \x1b (or \u001b) followed by [ and ending with m
    // Also handles bare [0m, [2m, [32m etc. that may appear without ESC
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c == '\x1b' || c == '\u{001b}' {
            // Skip escape sequence: ESC [ ... m
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                // Skip until 'm' or end of string
                while let Some(c) = chars.next() {
                    if c == 'm' {
                        break;
                    }
                }
            }
        } else if c == '[' {
            // Check if this is a bare ANSI code like [0m or [32m
            let mut is_ansi = true;
            let mut peek_chars: Vec<char> = Vec::new();
            
            // Peek ahead to check pattern: digits followed by 'm'
            loop {
                match chars.peek() {
                    Some(&d) if d.is_ascii_digit() => {
                        peek_chars.push(*chars.peek().unwrap());
                        chars.next();
                    }
                    Some(&';') => {
                        // Multiple codes like [0;32m
                        peek_chars.push(*chars.peek().unwrap());
                        chars.next();
                    }
                    Some(&'m') if !peek_chars.is_empty() => {
                        // Found [NNm pattern - skip the 'm' and continue
                        chars.next();
                        break;
                    }
                    _ => {
                        // Not an ANSI code, put chars back
                        is_ansi = false;
                        break;
                    }
                }
            }
            
            if !is_ansi {
                // Not ANSI, output the '[' and any peeked chars
                result.push('[');
                for ch in peek_chars {
                    result.push(ch);
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Parse log lines from child process stdout
pub fn parse_log_line(line: &str) -> Option<LogEntry> {
    // Try to parse structured log format: "2026-01-16T16:49:29.506004Z  INFO fortify_orchestrator: Message"
    // Also handles: "Jan 16 18:25:26.605 [notice] Bootstrapped 100%"
    
    // First strip any ANSI escape codes
    let line = strip_ansi_codes(line);
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    
    // Skip very short lines that are just fragments from ANSI formatting
    // These are typically bare values like "true", "sigilll", "30" 
    if line.len() < 10 {
        // Allow Tor percentage lines like "10%" and status messages
        if !line.ends_with('%') && !line.contains("OK") && !line.contains("Done") {
            return None;
        }
    }

    // Look for level indicators
    let level = if line.contains(" ERROR ") || line.contains("ERR") || line.contains("[err]") {
        LogLevel::Error
    } else if line.contains(" WARN ") || line.contains("WRN") || line.contains("[warn]") {
        LogLevel::Warn
    } else if line.contains(" INFO ") || line.contains("INF") || line.contains("[notice]") {
        LogLevel::Info
    } else if line.contains(" DEBUG ") || line.contains("DBG") || line.contains("[debug]") {
        LogLevel::Debug
    } else if line.contains(" TRACE ") || line.contains("TRC") {
        LogLevel::Trace
    } else {
        LogLevel::Info
    };
    
    // Skip DEBUG and TRACE level logs to reduce noise in TUI
    if level == LogLevel::Debug || level == LogLevel::Trace {
        return None;
    }
    
    // Skip noisy patterns that don't provide useful info
    let noisy_patterns = [
        "Found binary",
        "target/release/",
        "target/debug/",
        "enabled=false, prefix=''",
        "OrchestratorConfig {",
    ];
    for pattern in noisy_patterns {
        if line.contains(pattern) {
            return None;
        }
    }

    // Try to extract source
    let source = if let Some(start) = line.find("fortify_") {
        let end = line[start..].find(':').unwrap_or(20);
        &line[start..start + end.min(30)]
    } else if line.contains("Tor") || line.contains("[notice]") || line.contains("Bootstrapped") {
        "tor"
    } else {
        "system"
    };

    // Extract message, stripping timestamp and level info
    // Format 1: "2026-01-16T16:49:29.506004Z  INFO fortify_orchestrator: Message"
    // Format 2: "Jan 16 18:25:26.605 [notice] Message"
    let message = extract_message(line);

    Some(LogEntry::from_source(level, source, &message))
}

/// Extract just the message part, stripping timestamps and log level indicators
fn extract_message(line: &str) -> String {
    // Try to find message after "fortify_*: "
    if let Some(idx) = line.find("fortify_") {
        if let Some(colon_idx) = line[idx..].find(": ") {
            return line[idx + colon_idx + 2..].to_string();
        }
    }
    
    // Try to find message after "[notice] ", "[warn] ", etc.
    for marker in &["[notice] ", "[warn] ", "[err] ", "[debug] "] {
        if let Some(idx) = line.find(marker) {
            return line[idx + marker.len()..].to_string();
        }
    }
    
    // Try to find message after " INFO ", " WARN ", etc.
    for marker in &[" INFO ", " WARN ", " ERROR ", " DEBUG ", " TRACE "] {
        if let Some(idx) = line.find(marker) {
            // Skip to after the source: part
            let rest = &line[idx + marker.len()..];
            if let Some(colon_idx) = rest.find(": ") {
                return rest[colon_idx + 2..].to_string();
            }
            return rest.to_string();
        }
    }
    
    // If line starts with timestamp pattern, try to strip it
    // Pattern: "Jan 16 18:25:26.605" or "2026-01-16T18:25:26"
    if line.len() > 24 {
        // Check for ISO timestamp
        if line.chars().nth(4) == Some('-') && line.chars().nth(10) == Some('T') {
            // ISO format: skip past "2026-01-16T16:49:29.506004Z  "
            if let Some(z_idx) = line.find('Z') {
                let rest = line[z_idx + 1..].trim_start();
                if !rest.is_empty() {
                    return rest.to_string();
                }
            }
        }
        // Check for "Mon DD HH:MM:SS" format  
        let first_word: String = line.chars().take_while(|c| c.is_alphabetic()).collect();
        if matches!(first_word.as_str(), "Jan" | "Feb" | "Mar" | "Apr" | "May" | "Jun" | 
                                          "Jul" | "Aug" | "Sep" | "Oct" | "Nov" | "Dec") {
            // Find the space after the time (after .NNN or after SS)
            // "Jan 16 18:25:26.605 [notice] message"
            if let Some(bracket_idx) = line.find('[') {
                if let Some(close_idx) = line[bracket_idx..].find(']') {
                    let after_bracket = &line[bracket_idx + close_idx + 1..].trim_start();
                    if !after_bracket.is_empty() {
                        return after_bracket.to_string();
                    }
                }
            }
            // No brackets, just take everything after the time
            if line.len() > 20 {
                return line[20..].trim_start().to_string();
            }
        }
    }
    
    // Fallback: return the original line
    line.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_buffer() {
        let mut buf = LogBuffer::new(3);
        buf.push(LogEntry::info("one"));
        buf.push(LogEntry::info("two"));
        buf.push(LogEntry::info("three"));
        buf.push(LogEntry::info("four"));
        
        assert_eq!(buf.len(), 3);
        assert_eq!(buf.entries.front().unwrap().message, "two");
    }

    #[test]
    fn test_parse_log() {
        let line = "2026-01-16T16:49:29.506004Z  INFO fortify_orchestrator: Starting";
        let entry = parse_log_line(line).unwrap();
        assert_eq!(entry.level, LogLevel::Info);
    }
}
