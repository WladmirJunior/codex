//! Hide successful tools and retain bounded failure identities in focus mode.

use super::*;
use unicode_width::UnicodeWidthStr;

pub(crate) fn focus_tool_summary(tool: &str, status: &str, width: u16) -> Vec<HyperlinkLine> {
    if status == "completed" || width == 0 {
        return Vec::new();
    }
    let tool = tool.split_whitespace().collect::<Vec<_>>().join(" ");
    let available = usize::from(width).saturating_sub(status.width() + 4);
    let (prefix, rest, _) = take_prefix_by_width(&tool, available);
    let tool = if rest.is_empty() {
        prefix
    } else {
        let (prefix, _, _) = take_prefix_by_width(&tool, available.saturating_sub(1));
        format!("{prefix}…")
    };
    let text = format!("• {tool}: {status}");
    let (text, _, _) = take_prefix_by_width(&text, usize::from(width));
    let line = Line::from(text.red());
    plain_hyperlink_lines(vec![line])
}

pub(crate) fn focus_tool_failure_summary(
    tool: &str,
    exit_code: Option<i32>,
    output: impl IntoIterator<Item = impl AsRef<str>>,
    width: u16,
) -> Vec<HyperlinkLine> {
    let mut status = exit_code.map_or_else(
        || "failed".to_string(),
        |code| format!("failed (exit {code})"),
    );
    if let Some(detail) = output
        .into_iter()
        .map(|line| codex_ansi_escape::ansi_escape_line(line.as_ref()).to_string())
        .find(|line| !line.trim().is_empty())
    {
        let detail = detail.split_whitespace().collect::<Vec<_>>().join(" ");
        let available = usize::from(width).saturating_sub(tool.width() + status.width() + 6);
        let (prefix, rest, _) = take_prefix_by_width(&detail, available);
        let detail = if rest.is_empty() || available == 0 {
            prefix
        } else {
            let (prefix, _, _) = take_prefix_by_width(&detail, available.saturating_sub(1));
            format!("{prefix}…")
        };
        if !detail.is_empty() {
            status.push_str(": ");
            status.push_str(&detail);
        }
    }
    focus_tool_summary(tool, &status, width)
}

#[derive(Debug)]
pub(crate) struct FocusToolResultCell {
    pub(crate) full: PlainHistoryCell,
    pub(crate) tool: &'static str,
    pub(crate) status: &'static str,
    pub(crate) artifact: Option<Line<'static>>,
    pub(crate) exit_code: Option<i32>,
    pub(crate) error_detail: Option<String>,
}

impl HistoryCell for FocusToolResultCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        self.full.display_lines(width)
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        self.full.raw_lines()
    }

    fn focus_hyperlink_lines(&self, width: u16) -> Vec<HyperlinkLine> {
        let mut lines = if self.status == "failed" {
            focus_tool_failure_summary(
                self.tool,
                self.exit_code,
                self.error_detail.as_deref().unwrap_or_default().lines(),
                width,
            )
        } else {
            focus_tool_summary(self.tool, self.status, width)
        };
        lines.extend(self.artifact.iter().cloned().map(HyperlinkLine::new));
        lines
    }
}
