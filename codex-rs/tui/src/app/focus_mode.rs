//! Local, reversible transcript presentation. Never rewrites stored cells or server history.

use super::*;
use crate::legacy_core::config::edit::ConfigEdit;

impl App {
    pub(super) async fn toggle_focus_mode(&mut self, tui: &mut tui::Tui) {
        let enabled = !self.local_settings.tui.focus_mode;
        self.local_settings.tui.focus_mode = enabled;
        self.chat_widget.set_focus_mode(enabled);
        self.config.tui_focus_mode = enabled;
        let raw = self.chat_widget.raw_output_mode();
        self.apply_raw_output_mode(tui, raw, /*notify*/ false);
        let notice = if enabled {
            "Focus view on. Ctrl+T shows transcript details; /raw takes precedence."
        } else {
            "Focus view off. Standard chat view restored."
        };
        self.chat_widget
            .add_info_message(notice.into(), /*hint*/ None);
        if let Err(err) =
            ConfigEditsBuilder::for_config_path(self.local_settings.user_config_path.as_path())
                .with_edits([ConfigEdit::SetPath {
                    segments: vec!["tui".into(), "focus_mode".into()],
                    value: enabled.into(),
                }])
                .apply()
                .await
        {
            self.chat_widget.add_error_message(format!(
                "Focus view changed for this session, but could not save preference: {err}"
            ));
        }
    }
}
