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

#[derive(Debug)]
pub(crate) struct FocusToolResultCell {
    pub(crate) full: PlainHistoryCell,
    pub(crate) tool: &'static str,
    pub(crate) status: &'static str,
    pub(crate) artifact: Option<Line<'static>>,
}

impl HistoryCell for FocusToolResultCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        self.full.display_lines(width)
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        self.full.raw_lines()
    }

    fn focus_hyperlink_lines(&self, width: u16) -> Vec<HyperlinkLine> {
        let mut lines = focus_tool_summary(self.tool, self.status, width);
        lines.extend(self.artifact.iter().cloned().map(HyperlinkLine::new));
        lines
    }
}
