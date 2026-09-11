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
    assert!(
        cell.display_lines_for_mode(/*width*/ 80, HistoryRenderMode::Focus)
            .is_empty()
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
    let raw_before = cell.raw_lines();
    let text = before
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains("DETAILED_ARGUMENT"));
    assert!(text.contains("DETAILED_RESULT"));
    let focused = cell.display_lines_for_mode(/*width*/ 80, HistoryRenderMode::Focus);
    assert!(focused.is_empty());
    insta::assert_snapshot!(
        focused
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert_eq!(cell.transcript_lines(/*width*/ 80), before);
    assert_eq!(cell.raw_lines(), raw_before);
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
        assert!(cell.focus_hyperlink_lines(/*width*/ 80).is_empty());
        if let Some(code) = exit_code {
            cell.complete_call(
                "fixture-call",
                CommandOutput::new(code, "\nPermission denied\nDETAILED_OUTPUT".into()),
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
        let full = cell
            .transcript_lines(/*width*/ 80)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        match exit_code {
            Some(_) => assert_eq!(focused, ""),
            None => assert_eq!(focused, "• exec_command: interrupted"),
        }
        assert!(full.contains("DETAILED_COMMAND"));
        if exit_code.is_some() {
            assert!(full.contains("DETAILED_OUTPUT"));
        }
        snapshots.extend(focused.lines().map(str::to_owned));
    }
    insta::assert_snapshot!(snapshots.join("\n"));
}

#[test]
fn focus_exec_failure_hides_output_but_keeps_full_transcript() {
    let error = "rg: missing.rs: No such file or directory (os error 2)";
    let output = format!(
        "total 40\nFilesystem Size Used Avail\ne7420c63012fc19c8f6957f271dc56f49a788557\n{}\u{1b}[31m{error}\u{1b}[0m\nFINAL_OUTPUT",
        "DETAILED_OUTPUT\n".repeat(100)
    );
    let mut cell = crate::exec_cell::new_active_exec_command(
        "fixture-call".into(),
        vec!["rg".into(), "needle".into(), "missing.rs".into()],
        Vec::new(),
        codex_app_server_protocol::CommandExecutionSource::UnifiedExecStartup,
        /*interaction_input*/ None,
        /*animations_enabled*/ false,
    );
    cell.complete_call(
        "fixture-call",
        CommandOutput::new(2, output),
        Duration::ZERO,
    );
    for width in [120, 80, 40] {
        let lines = cell.display_lines_for_mode(width, HistoryRenderMode::Focus);
        assert_eq!(lines, Vec::<Line<'static>>::new());
        let text = cell
            .transcript_lines(width)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("total 40"));
        assert!(text.contains("Filesystem Size Used Avail"));
        assert_eq!(text.matches("DETAILED_OUTPUT").count(), 100);
        assert!(text.contains("FINAL_OUTPUT"));
        assert!(!text.contains("… +"));
        if width == 120 {
            assert!(text.contains(error));
        }
    }
    assert!(
        cell.raw_lines()
            .iter()
            .any(|line| line.to_string().contains("DETAILED_OUTPUT"))
    );
}

#[test]
fn focus_unicode_exec_error_is_preserved_outside_focus() {
    let mut cell = crate::exec_cell::new_active_exec_command(
        "fixture-call".into(),
        vec!["rg".into(), "missing.rs".into()],
        Vec::new(),
        codex_app_server_protocol::CommandExecutionSource::UnifiedExecStartup,
        /*interaction_input*/ None,
        /*animations_enabled*/ false,
    );
    cell.complete_call(
        "fixture-call",
        CommandOutput::new(/*exit_code*/ 2, "错误: 文件不存在".repeat(30)),
        Duration::ZERO,
    );
    for width in [40, 80] {
        let lines = cell.display_lines_for_mode(width, HistoryRenderMode::Focus);
        assert_eq!(lines, Vec::<Line<'static>>::new());
        let text = cell
            .transcript_lines(width)
            .iter()
            .map(ToString::to_string)
            .collect::<String>();
        assert_eq!(text.matches("错误").count(), 30);
        assert!(!text.contains('…'));
    }
}

#[test]
fn focus_mixed_exec_group_hides_completed_calls() {
    let mut cell = crate::exec_cell::new_active_exec_command(
        "success".into(),
        vec!["cat".into(), "SUCCESS_COMMAND".into()],
        Vec::new(),
        codex_app_server_protocol::CommandExecutionSource::Agent,
        /*interaction_input*/ None,
        /*animations_enabled*/ false,
    );
    cell.complete_call(
        "success",
        CommandOutput::new(/*exit_code*/ 0, "SUCCESS_OUTPUT".into()),
        Duration::ZERO,
    );
    let mut failed = crate::exec_cell::new_active_exec_command(
        "failed".into(),
        vec!["cat".into(), "FAILED_COMMAND".into()],
        Vec::new(),
        codex_app_server_protocol::CommandExecutionSource::Agent,
        /*interaction_input*/ None,
        /*animations_enabled*/ false,
    );
    failed.complete_call(
        "failed",
        CommandOutput::new(/*exit_code*/ 2, "total 40\nFAILED_OUTPUT".into()),
        Duration::ZERO,
    );
    cell.calls.extend(failed.calls);
    assert_eq!(
        cell.display_lines_for_mode(/*width*/ 80, HistoryRenderMode::Focus),
        Vec::<Line<'static>>::new()
    );
    assert!(
        cell.raw_lines()
            .iter()
            .any(|line| line.to_string().contains("SUCCESS_OUTPUT"))
    );
}

#[test]
fn focus_keeps_explicit_user_shell_commands_visible() {
    let mut cell = crate::exec_cell::new_active_exec_command(
        "user-call".into(),
        vec!["echo".into(), "USER_COMMAND".into()],
        Vec::new(),
        codex_app_server_protocol::CommandExecutionSource::UserShell,
        /*interaction_input*/ None,
        /*animations_enabled*/ false,
    );
    assert_eq!(
        cell.focus_hyperlink_lines(/*width*/ 80),
        cell.display_hyperlink_lines(/*width*/ 80)
    );
    cell.complete_call(
        "user-call",
        CommandOutput::new(/*exit_code*/ 0, "USER_OUTPUT".into()),
        Duration::ZERO,
    );
    assert_eq!(
        cell.focus_hyperlink_lines(/*width*/ 80),
        cell.display_hyperlink_lines(/*width*/ 80)
    );
}

#[test]
fn focus_hides_patch_and_background_interactions_but_retains_details() {
    let patch = new_patch_event(
        HashMap::from([(
            test_path_buf("/tmp/fixture.rs"),
            FileChange::Add {
                content: "fixture".into(),
            },
        )]),
        &test_path_buf("/tmp"),
    );
    let cells: Vec<Box<dyn HistoryCell>> = vec![
        Box::new(patch),
        Box::new(new_unified_exec_interaction(
            Some("sleep 10".into()),
            String::new(),
        )),
        Box::new(new_unified_exec_interaction(
            Some("cat".into()),
            "input".into(),
        )),
    ];
    for cell in cells {
        let before = cell.display_lines(/*width*/ 80);
        let raw = cell.raw_lines();
        assert!(!before.is_empty());
        assert!(cell.focus_hyperlink_lines(/*width*/ 80).is_empty());
        assert_eq!(cell.display_lines(/*width*/ 80), before);
        assert_eq!(cell.raw_lines(), raw);
    }
    let failure = new_patch_apply_failure("Permission denied".into());
    assert!(!failure.focus_hyperlink_lines(/*width*/ 80).is_empty());
}

#[test]
fn focus_hides_routine_subagent_activity_but_keeps_interruptions() {
    use codex_app_server_protocol::SubAgentActivityKind;
    for kind in [
        SubAgentActivityKind::Started,
        SubAgentActivityKind::Interacted,
        SubAgentActivityKind::Completed,
        SubAgentActivityKind::Interrupted,
    ] {
        let cell = crate::multi_agents::sub_agent_activity_history_cell(
            &codex_app_server_protocol::ThreadItem::SubAgentActivity {
                id: "fixture-call".into(),
                kind,
                agent_thread_id: "00000000-0000-0000-0000-000000000001".into(),
                agent_path: "/root/worker".into(),
            },
        )
        .unwrap();
        let raw = cell.raw_lines();
        let focused = cell.focus_hyperlink_lines(/*width*/ 80);
        if kind == SubAgentActivityKind::Interrupted {
            assert_eq!(focused, cell.display_hyperlink_lines(/*width*/ 80));
        } else {
            assert!(focused.is_empty());
        }
        assert!(!raw.is_empty());
        assert_eq!(cell.raw_lines(), raw);
    }
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
    assert_eq!(focused.lines().count(), 1);
    assert!(!focused.contains("DETAILED_IMAGE_PROMPT"));
    assert!(
        cell.transcript_lines(/*width*/ 80)
            .iter()
            .any(|line| line.to_string().contains("DETAILED_IMAGE_PROMPT"))
    );
}
