use std::collections::HashMap;

use codex_app_server_protocol::AdditionalContextEntry;
use codex_app_server_protocol::AdditionalContextKind;

pub(super) fn context(focus_mode: bool) -> HashMap<String, AdditionalContextEntry> {
    let value = if focus_mode {
        "Focus mode is enabled for this turn. This replaces routine preamble and progress-update preferences: omit announcements, acknowledgments, and commentary that merely describe the next action. For an authorized implementation task, continue using tools until the requested work is completed and verified, or a genuine blocker requires user input; do not end the turn merely to promise the next step. Give one concise final answer with the outcome and any remaining blocker. Still ask necessary blocking questions and preserve approval, safety, and current collaboration-mode requirements, including plan-mode restrictions. Do not announce this policy."
    } else {
        "Focus mode is disabled for this turn. The previous Focus-mode communication policy no longer applies. Follow the normal communication instructions and current collaboration mode."
    };
    HashMap::from([(
        "tui.focus_mode".to_string(),
        AdditionalContextEntry {
            value: value.to_string(),
            kind: AdditionalContextKind::Application,
        },
    )])
}
