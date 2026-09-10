use super::*;
use crate::history_cell::HistoryRenderMode;
use crate::history_cell::McpInvocation;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn focus_toggle_reflows_existing_cells_and_persists_without_rewriting() -> Result<()> {
    let (mut app, _app_event_rx, _op_rx) = make_test_app_with_channels().await;
    let mut tui = crate::tui::test_support::make_test_tui()?;
    let mut cell = history_cell::new_active_mcp_tool_call(
        "fixture".into(),
        McpInvocation {
            server: "fixture".into(),
            tool: "inspect".into(),
            arguments: Some(serde_json::json!({"payload": "DETAILED_ARGUMENT"})),
        },
        /*animations_enabled*/ false,
    );
    cell.mark_failed();
    app.insert_history_cell(&mut tui, Box::new(cell));
    let original_cells = app.transcript_cells.clone();
    let before = app.render_transcript_lines_for_reflow(/*width*/ 80).lines;
    app.toggle_focus_mode(&mut tui).await;
    let focused = app.render_transcript_lines_for_reflow(/*width*/ 80).lines;
    assert_ne!(before, focused);
    assert_eq!(
        app.chat_widget.history_render_mode(),
        HistoryRenderMode::Focus
    );
    let saved: toml::Value = toml::from_str(&std::fs::read_to_string(
        &app.local_settings.user_config_path,
    )?)?;
    assert_eq!(saved["tui"]["focus_mode"].as_bool(), Some(true));
    app.chat_widget.set_raw_output_mode(/*enabled*/ true);
    assert_eq!(
        app.chat_widget.history_render_mode(),
        HistoryRenderMode::Raw
    );
    app.chat_widget.set_raw_output_mode(/*enabled*/ false);
    assert_eq!(
        app.chat_widget.history_render_mode(),
        HistoryRenderMode::Focus
    );
    app.toggle_focus_mode(&mut tui).await;
    assert_eq!(
        app.render_transcript_lines_for_reflow(/*width*/ 80).lines,
        before
    );
    assert!(
        app.transcript_cells
            .iter()
            .zip(&original_cells)
            .all(|(a, b)| Arc::ptr_eq(a, b))
    );
    let saved: toml::Value = toml::from_str(&std::fs::read_to_string(
        &app.local_settings.user_config_path,
    )?)?;
    assert_eq!(saved["tui"]["focus_mode"].as_bool(), Some(false));
    insta::assert_snapshot!(
        focused
            .iter()
            .map(|line| line.line.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    );
    Ok(())
}
