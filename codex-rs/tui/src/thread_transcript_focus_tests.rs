use super::*;
use crate::history_cell::HistoryRenderMode;
use crate::test_support::PathBufExt;
use pretty_assertions::assert_eq;

#[test]
fn focus_paginated_messages_preserve_phase_and_restore_details() {
    use codex_protocol::models::MessagePhase;
    let items = [
        (MessagePhase::Commentary, "Checking files"),
        (MessagePhase::FinalAnswer, "Fixed the issue"),
    ]
    .into_iter()
    .map(|(phase, text)| ThreadItem::AgentMessage {
        id: text.into(),
        text: text.into(),
        phase: Some(phase),
        memory_citation: None,
        delivery: None,
        questions: None,
    });
    let cells = thread_items_to_transcript_cells(
        None,
        &crate::test_support::test_path_buf("/tmp").abs(),
        items,
        RawReasoningVisibility::Visible,
        None,
    );
    let render = |mode| {
        cells
            .iter()
            .flat_map(|cell| cell.display_lines_for_mode(80, mode))
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
    };
    let before = render(HistoryRenderMode::Rich);
    assert_eq!(render(HistoryRenderMode::Focus), vec!["• Fixed the issue"]);
    assert_eq!(
        render(HistoryRenderMode::Raw),
        vec!["Checking files", "Fixed the issue"]
    );
    assert_eq!(render(HistoryRenderMode::Rich), before);
}

#[test]
fn focus_paginated_tool_activity_is_quiet_and_preserves_failures() {
    let fixtures = [
        (
            serde_json::json!({"type": "fileChange", "id": "patch", "changes": [], "status": "completed"}),
            false,
        ),
        (
            serde_json::json!({"type": "fileChange", "id": "patch-error", "changes": [], "status": "failed"}),
            true,
        ),
        (
            serde_json::json!({"type": "webSearch", "id": "search", "query": "fixture query", "action": null, "results": null}),
            false,
        ),
        (
            serde_json::json!({"type": "subAgentActivity", "id": "start", "kind": "started", "agentThreadId": "worker", "agentPath": "/root/worker"}),
            false,
        ),
        (
            serde_json::json!({"type": "subAgentActivity", "id": "interaction", "kind": "interacted", "agentThreadId": "worker", "agentPath": "/root/worker"}),
            false,
        ),
        (
            serde_json::json!({"type": "subAgentActivity", "id": "done", "kind": "completed", "agentThreadId": "worker", "agentPath": "/root/worker"}),
            false,
        ),
        (
            serde_json::json!({"type": "subAgentActivity", "id": "interrupted", "kind": "interrupted", "agentThreadId": "worker", "agentPath": "/root/worker"}),
            true,
        ),
        (
            serde_json::json!({"type": "collabAgentToolCall", "id": "collab", "tool": "wait", "status": "completed", "senderThreadId": "root", "receiverThreadIds": ["worker"], "prompt": null, "model": null, "reasoningEffort": null, "agentsStates": {"worker": {"status": "completed", "message": "done"}}}),
            false,
        ),
        (
            serde_json::json!({"type": "collabAgentToolCall", "id": "collab-error", "tool": "wait", "status": "completed", "senderThreadId": "root", "receiverThreadIds": ["worker"], "prompt": null, "model": null, "reasoningEffort": null, "agentsStates": {"worker": {"status": "errored", "message": "failed"}}}),
            true,
        ),
        (
            serde_json::json!({"type": "collabAgentToolCall", "id": "collab-failed", "tool": "wait", "status": "failed", "senderThreadId": "root", "receiverThreadIds": [], "prompt": null, "model": null, "reasoningEffort": null, "agentsStates": {}}),
            true,
        ),
    ];
    let mut visible = Vec::new();
    for (fixture, requires_attention) in fixtures {
        let item = serde_json::from_value(fixture).unwrap();
        let cell = fallback_transcript_cell(&item).unwrap();
        let full = cell.display_lines(/*width*/ 80);
        let raw = cell.raw_lines();
        assert!(!full.is_empty());
        if requires_attention {
            assert_eq!(
                cell.display_lines_for_mode(/*width*/ 80, HistoryRenderMode::Focus),
                full
            );
            visible.extend(full.iter().map(ToString::to_string));
        } else {
            assert!(
                cell.display_lines_for_mode(/*width*/ 80, HistoryRenderMode::Focus)
                    .is_empty()
            );
        }
        assert_eq!(cell.display_lines(/*width*/ 80), full);
        assert_eq!(cell.raw_lines(), raw);
    }
    insta::assert_snapshot!(visible.join("\n"), @"
    file changes: Failed · 0 changes
    Interrupted `/root/worker`
    agent tool: Wait · Completed
    agent tool: Wait · Failed
    ");
}

#[test]
fn focus_paginated_errors_are_quiet_like_live_errors() {
    for diagnostic in [
        "rg: missing.rs: No such file or directory (os error 2)",
        "错误: 文件不存在，无法读取文件",
    ] {
        let output = format!(
            "total 40\nFilesystem Size Used Avail\n{}\u{1b}[31m{diagnostic}\u{1b}[0m\nFINAL_OUTPUT",
            "DETAILED_OUTPUT\n".repeat(100)
        );
        let item: ThreadItem = serde_json::from_value(serde_json::json!({
            "type": "commandExecution", "id": "fixture", "command": "rg needle missing.rs",
            "cwd": "/tmp", "processId": null, "status": "failed", "source": "unifiedExecStartup",
            "commandActions": [], "aggregatedOutput": output, "exitCode": 2, "durationMs": 1
        }))
        .unwrap();
        let paginated = fallback_transcript_cell(&item).unwrap();
        let original = paginated.raw_lines();
        let mut live = crate::exec_cell::new_active_exec_command(
            "fixture".into(),
            vec!["rg".into(), "needle".into(), "missing.rs".into()],
            Vec::new(),
            codex_app_server_protocol::CommandExecutionSource::UnifiedExecStartup,
            /*interaction_input*/ None,
            /*animations_enabled*/ false,
        );
        live.complete_call(
            "fixture",
            crate::exec_cell::CommandOutput::new(/*exit_code*/ 2, output),
            std::time::Duration::ZERO,
        );
        for width in [40, 80, 120] {
            let focused = paginated.display_lines_for_mode(width, HistoryRenderMode::Focus);
            assert_eq!(focused, Vec::<Line<'static>>::new());
            let live_focused = live.display_lines_for_mode(width, HistoryRenderMode::Focus);
            assert_eq!(live_focused, Vec::<Line<'static>>::new());
            for lines in [paginated.display_lines(width), live.transcript_lines(width)] {
                let text = lines
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("\n");
                assert!(text.contains("total 40"));
                assert_eq!(text.matches("DETAILED_OUTPUT").count(), 100);
                assert!(text.contains("FINAL_OUTPUT"));
                assert!(!text.contains("… +"));
            }
            if width == 120 {
                assert!(
                    paginated
                        .display_lines(width)
                        .iter()
                        .any(|line| line.to_string().contains(diagnostic))
                );
            }
        }
        assert_eq!(paginated.raw_lines(), original);
        assert!(
            original
                .iter()
                .any(|line| line.to_string().contains("DETAILED_OUTPUT"))
        );
    }
}

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
        Vec::<String>::new()
    );
    assert_eq!(cell.transcript_lines(/*width*/ 80), before);
    assert_eq!(
        cell.display_lines_for_mode(/*width*/ 80, HistoryRenderMode::Rich),
        before
    );
}
