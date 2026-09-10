use super::*;
use codex_protocol::mcp::CallToolResult;
use pretty_assertions::assert_eq;
use serde_json::json;

#[test]
fn focus_completed_mcp_retains_full_transcript() {
    let mut cell = new_active_mcp_tool_call(
        "fixture-call".into(),
        McpInvocation {
            server: "fixture".into(),
            tool: "inspect".into(),
            arguments: Some(json!({"payload": "DETAILED_ARGUMENT"})),
        },
        /*animations_enabled*/ false,
    );
    let original = cell.display_lines(/*width*/ 80);
    assert_eq!(
        cell.display_lines_for_mode(/*width*/ 80, HistoryRenderMode::Focus),
        original
    );
    cell.complete(
        Duration::from_millis(1),
        Ok(CallToolResult {
            content: vec![json!({"type": "text", "text": "DETAILED_RESULT\nsecond line"})],
            is_error: None,
            structured_content: None,
            meta: None,
        }),
    );
    let before = cell.transcript_lines(/*width*/ 80);
    let text = before
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains("DETAILED_ARGUMENT"));
    assert!(text.contains("DETAILED_RESULT"));
    let focused = cell.display_lines_for_mode(/*width*/ 80, HistoryRenderMode::Focus);
    assert_eq!(
        focused.iter().map(ToString::to_string).collect::<Vec<_>>(),
        vec!["• fixture.inspect: completed"]
    );
    insta::assert_snapshot!(
        focused
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert_eq!(cell.transcript_lines(/*width*/ 80), before);
    assert_eq!(
        cell.display_lines_for_mode(/*width*/ 80, HistoryRenderMode::Rich),
        cell.display_lines(/*width*/ 80)
    );
}

#[test]
fn focus_mcp_failures_and_cancellation_keep_identity_without_payloads() {
    let mut snapshots = Vec::new();
    for outcome in [
        "result_error",
        "transport_error",
        "cancelled",
        "approval_cancelled",
    ] {
        let mut cell = new_active_mcp_tool_call(
            "fixture-call".into(),
            McpInvocation {
                server: "fixture".into(),
                tool: "inspect".into(),
                arguments: Some(json!({"payload": "DETAILED_ARGUMENT"})),
            },
            /*animations_enabled*/ false,
        );
        match outcome {
            "result_error" => {
                cell.complete(
                    Duration::ZERO,
                    Ok(CallToolResult {
                        content: vec![json!({"type": "text", "text": "DETAILED_ERROR"})],
                        is_error: Some(true),
                        structured_content: None,
                        meta: None,
                    }),
                );
            }
            "transport_error" => {
                cell.complete(Duration::ZERO, Err("DETAILED_ERROR".into()));
            }
            "approval_cancelled" => {
                cell.complete(Duration::ZERO, Err("user cancelled MCP tool call".into()));
            }
            _ => cell.mark_failed(),
        }
        let text = cell
            .display_lines_for_mode(/*width*/ 80, HistoryRenderMode::Focus)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(
            text,
            format!(
                "• fixture.inspect: {}",
                match outcome {
                    "cancelled" => "interrupted",
                    "approval_cancelled" => "cancelled",
                    _ => "failed",
                }
            )
        );
        let full = cell
            .transcript_lines(/*width*/ 80)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(full.contains("DETAILED_ARGUMENT"));
        if !outcome.ends_with("cancelled") {
            assert!(full.contains("DETAILED_ERROR"));
        }
        snapshots.push(text);
    }
    insta::assert_snapshot!(snapshots.join("\n"));
}

#[test]
fn focus_hides_reasoning_but_preserves_messages_and_safety_notices() {
    let reasoning = ReasoningSummaryCell::new(
        "Reasoning".into(),
        "DETAILED_REASONING".into(),
        &test_path_buf("/fixture"),
        /*transcript_only*/ false,
    );
    assert!(
        reasoning
            .display_lines_for_mode(/*width*/ 40, HistoryRenderMode::Focus)
            .is_empty()
    );
    assert!(
        reasoning
            .transcript_lines(/*width*/ 40)
            .iter()
            .any(|line| line.to_string().contains("DETAILED_REASONING"))
    );
    let notice = PlainHistoryCell::new(vec!["Approval required: inspect /fixture".into()]);
    assert_eq!(
        notice.display_lines_for_mode(/*width*/ 40, HistoryRenderMode::Focus),
        notice.display_lines(/*width*/ 40)
    );
    let safety = new_safety_access_block_event();
    assert_eq!(
        safety.focus_hyperlink_lines(/*width*/ 40),
        safety.display_hyperlink_lines(/*width*/ 40)
    );
}

#[test]
fn focus_exec_completed_failed_cancelled_and_running() {
    let mut snapshots = Vec::new();
    for exit_code in [Some(0), Some(7), None] {
        let mut cell = crate::exec_cell::new_active_exec_command(
            "fixture-call".into(),
            vec!["echo".into(), "DETAILED_COMMAND".into()],
            Vec::new(),
            codex_app_server_protocol::CommandExecutionSource::UnifiedExecStartup,
            /*interaction_input*/ None,
            /*animations_enabled*/ false,
        );
        assert_eq!(
            cell.focus_hyperlink_lines(/*width*/ 80),
            cell.display_hyperlink_lines(/*width*/ 80)
        );
        if let Some(code) = exit_code {
            cell.complete_call(
                "fixture-call",
                CommandOutput::new(code, "DETAILED_OUTPUT".into()),
                Duration::ZERO,
            );
        } else {
            cell.mark_failed();
        }
        let focused = cell
            .display_lines_for_mode(/*width*/ 80, HistoryRenderMode::Focus)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!focused.contains("DETAILED_"));
        let expected = match exit_code {
            Some(0) => "completed",
            Some(_) => "failed (exit 7)",
            None => "interrupted",
        };
        assert_eq!(focused, format!("• exec_command: {expected}"));
        let full = cell
            .transcript_lines(/*width*/ 80)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(full.contains("DETAILED_COMMAND"));
        if exit_code.is_some() {
            assert!(full.contains("DETAILED_OUTPUT"));
        }
        snapshots.push(focused);
    }
    insta::assert_snapshot!(snapshots.join("\n"));
}

#[test]
fn focus_unicode_identity_is_bounded_on_narrow_terminals() {
    let mut snapshots = Vec::new();
    for width in [80, 24, 12, 0] {
        let lines = focus_tool_summary("workspace.分析_arquivo_muito_longo", "failed", width);
        assert!(
            lines
                .iter()
                .all(|line| line.line.width() <= usize::from(width))
        );
        snapshots.push(format!(
            "{width}: {}",
            lines
                .iter()
                .map(|line| line.line.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    insta::assert_snapshot!(snapshots.join("\n"));
}

#[test]
fn focus_image_artifact_remains_accessible() {
    let cell = new_image_generation_call(
        "fixture-image".into(),
        "completed",
        Some("DETAILED_IMAGE_PROMPT".into()),
        Some(test_path_buf("/tmp/generated-image.png").abs()),
    );
    let focused = cell
        .display_lines_for_mode(/*width*/ 80, HistoryRenderMode::Focus)
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(focused.contains("Saved to:"));
    assert!(focused.contains("generated-image.png"));
    assert!(!focused.contains("DETAILED_IMAGE_PROMPT"));
    assert!(
        cell.transcript_lines(/*width*/ 80)
            .iter()
            .any(|line| line.to_string().contains("DETAILED_IMAGE_PROMPT"))
    );
}
