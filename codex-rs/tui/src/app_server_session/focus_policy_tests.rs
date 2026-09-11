use super::*;
use crate::legacy_core::config::ConfigBuilder;
use codex_protocol::config_types::CollaborationMode;
use codex_protocol::config_types::ModeKind;
use codex_protocol::config_types::Settings;
use core_test_support::responses;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn focus_policy_reaches_model_and_revokes_without_rewriting_history() -> Result<()> {
    let server = responses::start_mock_server().await;
    let response = responses::mount_sse_sequence(
        &server,
        (0..3)
            .map(|_| {
                responses::sse(vec![
                    responses::ev_response_created("response"),
                    responses::ev_completed("response"),
                ])
            })
            .collect(),
    )
    .await;
    let home = tempfile::tempdir()?;
    let base_url = server.uri();
    std::fs::write(
        home.path().join("config.toml"),
        format!(
            r#"
model = "gpt-5.2"
model_provider = "focus-test"
developer_instructions = "Preserve this developer requirement."
[model_providers.focus-test]
name = "OpenAI"
base_url = "{base_url}/v1"
wire_api = "responses"
request_max_retries = 0
stream_max_retries = 0
"#
        ),
    )?;
    let config = ConfigBuilder::default()
        .codex_home(home.path().to_path_buf())
        .build()
        .await?;
    let mut app_server = crate::start_embedded_app_server_for_picker(&config).await?;
    let started = app_server.start_thread(&config).await?;
    let collaboration = CollaborationMode {
        mode: ModeKind::Plan,
        settings: Settings {
            model: "gpt-5.2".into(),
            reasoning_effort: None,
            developer_instructions: Some("Preserve this collaboration requirement.".into()),
        },
    };
    for focus_mode in [true, true, false] {
        app_server
            .turn_start(
                started.session.thread_id,
                uuid::Uuid::new_v4().to_string(),
                vec![UserInput::Text {
                    text: "Continue the requested task.".into(),
                    text_elements: Vec::new(),
                }],
                config.cwd.to_path_buf(),
                /*approval_policy*/ None,
                /*approvals_reviewer*/ None,
                TurnPermissionsOverride::Preserve,
                config.permissions.user_visible_workspace_roots(),
                "gpt-5.2".into(),
                /*effort*/ None,
                /*summary*/ None,
                /*service_tier*/ None,
                Some(collaboration.clone()),
                /*personality*/ None,
                /*output_schema*/ None,
                focus_mode,
            )
            .await?;
        tokio::time::timeout(std::time::Duration::from_secs(/*secs*/ 30), async {
            while let Some(event) = app_server.next_event().await {
                if let AppServerEvent::ServerNotification(notification) = event
                    && let codex_app_server_protocol::ServerNotification::TurnCompleted(completed) =
                        *notification
                {
                    assert_eq!(
                        completed.turn.status,
                        codex_app_server_protocol::TurnStatus::Completed
                    );
                    return;
                }
            }
            panic!("app-server disconnected before completing the turn");
        })
        .await?;
    }
    let bodies = response
        .requests()
        .iter()
        .map(core_test_support::responses::ResponsesRequest::body_json)
        .collect::<Vec<_>>();
    assert_eq!(bodies.len(), 3);
    let focus_entries = bodies
        .iter()
        .map(|body| {
            body["input"]
                .as_array()
                .expect("input array")
                .iter()
                .filter(|item| {
                    item["role"] == "developer" && item.to_string().contains("<tui.focus_mode>")
                })
                .cloned()
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    assert_eq!(focus_entries[0], focus_entries[1]);
    assert_eq!(focus_entries[0].len(), 1);
    assert_eq!(focus_entries[2].len(), 2);
    assert_eq!(focus_entries[2][0], focus_entries[0][0]);
    assert!(
        focus_entries[2][1]
            .to_string()
            .contains("Focus mode is disabled")
    );
    for body in &bodies {
        let text = body.to_string();
        assert!(text.contains("Preserve this developer requirement."));
        assert!(text.contains("Preserve this collaboration requirement."));
    }
    app_server.shutdown().await?;
    Ok(())
}
