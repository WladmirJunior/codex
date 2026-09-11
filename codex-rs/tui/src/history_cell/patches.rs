//! Patch summaries and image-tool transcript helpers.

use super::*;
use codex_utils_path_uri::LegacyAppPathString;

#[derive(Debug)]
pub(crate) struct PatchHistoryCell {
    changes: HashMap<PathBuf, FileChange>,
    cwd: PathBuf,
    approval_preview: bool,
}

impl HistoryCell for PatchHistoryCell {
    fn focus_hyperlink_lines(&self, width: u16) -> Vec<HyperlinkLine> {
        if self.approval_preview {
            self.display_hyperlink_lines(width)
        } else {
            Vec::new()
        }
    }

    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        create_diff_summary(&self.changes, &self.cwd, width as usize)
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        plain_lines(create_diff_summary(
            &self.changes,
            &self.cwd,
            RAW_DIFF_SUMMARY_WIDTH,
        ))
    }
}
/// Create a new `PendingPatch` cell that lists the file‑level summary of
/// a proposed patch. The summary lines should already be formatted (e.g.
/// "A path/to/file.rs").
pub(crate) fn new_patch_event(
    changes: HashMap<PathBuf, FileChange>,
    cwd: &Path,
) -> PatchHistoryCell {
    PatchHistoryCell {
        changes,
        cwd: cwd.to_path_buf(),
        approval_preview: false,
    }
}

pub(crate) fn new_patch_approval_preview(
    changes: HashMap<PathBuf, FileChange>,
    cwd: &Path,
) -> PatchHistoryCell {
    PatchHistoryCell {
        approval_preview: true,
        ..new_patch_event(changes, cwd)
    }
}

pub(crate) fn new_patch_apply_failure(stderr: String) -> impl HistoryCell {
    let mut lines: Vec<Line<'static>> = Vec::new();

    // Failure title
    lines.push(Line::from("✘ Failed to apply patch".magenta().bold()));

    if !stderr.trim().is_empty() {
        let output = output_lines(
            Some(&CommandOutput::new(/*exit_code*/ 1, stderr)),
            OutputLinesParams {
                line_limit: TOOL_CALL_MAX_LINES,
                only_err: true,
                include_angle_pipe: true,
                include_prefix: true,
            },
        );
        lines.extend(output.lines);
    }

    FocusToolResultCell {
        full: PlainHistoryCell { lines },
        tool: "apply_patch",
        status: "failed",
        artifact: None,
        exit_code: None,
        error_detail: None,
    }
}

pub(crate) fn new_view_image_tool_call(path: LegacyAppPathString, cwd: &Path) -> impl HistoryCell {
    let display_path = path
        .to_inferred_path_uri()
        .and_then(|path| path.to_abs_path().ok())
        .map(|path| display_path_for(path.as_path(), cwd))
        .unwrap_or_else(|| path.into_string());

    let lines: Vec<Line<'static>> = vec![
        vec!["• ".dim(), "Viewed Image".bold()].into(),
        vec!["  └ ".dim(), display_path.dim()].into(),
    ];

    FocusToolResultCell {
        artifact: lines.last().cloned(),
        full: PlainHistoryCell { lines },
        tool: "view_image",
        status: "completed",
        exit_code: None,
        error_detail: None,
    }
}

pub(crate) fn new_image_generation_call(
    call_id: String,
    status: &str,
    revised_prompt: Option<String>,
    saved_path: Option<AbsolutePathBuf>,
) -> impl HistoryCell {
    let detail = revised_prompt.unwrap_or(call_id);
    let heading = if status == "failed" {
        vec!["✗ ".red().bold(), "Image generation failed".bold()].into()
    } else {
        vec!["• ".dim(), "Generated Image:".bold()].into()
    };
    let mut lines: Vec<Line<'static>> = vec![heading, vec!["  └ ".dim(), detail.dim()].into()];
    let has_saved_path = saved_path.is_some();
    if let Some(saved_path) = saved_path {
        let saved_path = Url::from_file_path(saved_path.as_path())
            .map(|url| url.to_string())
            .unwrap_or_else(|_| saved_path.display().to_string());
        lines.push(vec!["  └ ".dim(), "Saved to: ".dim(), saved_path.into()].into());
    }

    FocusToolResultCell {
        artifact: has_saved_path.then(|| lines.last().cloned()).flatten(),
        full: PlainHistoryCell { lines },
        tool: "image_generation",
        exit_code: None,
        error_detail: None,
        status: if status == "failed" {
            "failed"
        } else {
            "completed"
        },
    }
}
