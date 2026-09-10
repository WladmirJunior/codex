use super::*;
use crate::history_cell::HistoryRenderMode;
use crate::test_support::PathBufExt;
use pretty_assertions::assert_eq;

#[test]
fn focus_paginated_command_preserves_original_transcript() {
    let item: ThreadItem = serde_json::from_value(serde_json::json!({
        "type": "commandExecution", "id": "fixture", "command": "echo DETAILED_COMMAND",
        "cwd": "/tmp", "processId": null, "status": "completed", "source": "unifiedExecStartup",
        "commandActions": [], "aggregatedOutput": "DETAILED_OUTPUT", "exitCode": 0, "durationMs": 1
    }))
    .unwrap();
    let cells = thread_items_to_transcript_cells(
        None,
        &crate::test_support::test_path_buf("/tmp").abs(),
        vec![item],
        RawReasoningVisibility::Visible,
        None,
    );
    let cell = &cells[0];
    let before = cell.transcript_lines(/*width*/ 80);
    let full = before
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(full.contains("DETAILED_COMMAND"));
    assert!(full.contains("DETAILED_OUTPUT"));
    assert_eq!(
        cell.display_lines_for_mode(/*width*/ 80, HistoryRenderMode::Focus)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        vec!["• exec_command: completed"]
    );
    assert_eq!(cell.transcript_lines(/*width*/ 80), before);
    assert_eq!(
        cell.display_lines_for_mode(/*width*/ 80, HistoryRenderMode::Rich),
        before
    );
}
