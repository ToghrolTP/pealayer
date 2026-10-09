//! Application adapter for host-owned SFX, using the same effect editor and
//! command/catalog contracts as controller effects. Remote consumers mutate
//! only their authority's library, never their local filesystem.
use crate::{
    app::{ControllerEffectDraft, PealayerApp},
    mpv::sfx::{AudioEffect, Completion},
    platform::interop::{InteropCommand, WebControllerEffect},
};
impl PealayerApp {
    pub(crate) fn audio_previewing(&self, id: uuid::Uuid) -> bool {
        if let Some(snapshot) = crate::peer::client().and_then(|client| client.snapshot()) {
            snapshot
                .session
                .status
                .get("audio_preview_ids")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|ids| {
                    ids.iter()
                        .any(|value| value.as_str() == Some(id.to_string().as_str()))
                })
        } else {
            crate::mpv::sfx::previewing(id)
        }
    }
    pub(crate) fn audio_effects(&self) -> Vec<AudioEffect> {
        if let Some(snapshot) = crate::peer::client().and_then(|c| c.snapshot()) {
            snapshot.session.config.audio_effects.clone()
        } else {
            crate::platform::interop::get_live_config().audio_effects
        }
    }
    pub(crate) fn web_audio_effect(&self, effect: &AudioEffect) -> WebControllerEffect {
        WebControllerEffect {
            reference: effect.reference(),
            id: effect.id.to_string(),
            name: effect.name.clone(),
            icon: effect.icon.clone(),
            category: effect.group.clone(),
            description: effect.description.clone(),
            kind: "audio".into(),
            duration_ms: effect.duration_ms,
            duration_display: crate::duration::format_effect_duration_for_language(
                self.language,
                effect.duration_ms,
            ),
            action_count: 1,
            editable: true,
            lane: "audio".into(),
            program: serde_json::to_value(&effect.program).unwrap(),
            default_fps: None,
            default_pixels: None,
        }
    }
    pub(crate) fn select_audio_effect(&mut self, id: uuid::Uuid) {
        if let Some(effect) = self.audio_effects().into_iter().find(|e| e.id == id) {
            self.effect_library_selection = Some(effect.reference());
            self.effect_library_draft = ControllerEffectDraft {
                reference: effect.reference(),
                id: effect.id.to_string(),
                name: effect.name,
                category: effect.group,
                icon: effect.icon,
                description: effect.description,
                kind: "audio".into(),
                duration_ms: effect.duration_ms,
                program_json: serde_json::to_string_pretty(&effect.program).unwrap(),
                is_new: false,
                ..ControllerEffectDraft::default()
            };
        }
    }
    pub(crate) fn begin_audio_effect(&mut self) {
        self.effect_library_selection = None;
        self.effect_library_draft = ControllerEffectDraft {
            id: uuid::Uuid::new_v4().to_string(),
            kind: "audio".into(),
            icon: "speaker-high".into(),
            category: "Audio".into(),
            duration_ms: 0,
            program_json: serde_json::to_string(&crate::mpv::sfx::AudioProgram::default()).unwrap(),
            ..ControllerEffectDraft::default()
        };
        self.show_effect_library_editor = true;
    }
    pub(crate) fn route_audio_command(&self, command: InteropCommand) -> Result<bool, String> {
        if let Some(client) = crate::peer::client() {
            client.queue(
                "/api/player/command",
                serde_json::to_value(command).map_err(|e| e.to_string())?,
            )?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    pub(crate) fn process_audio_effect_results(&mut self) {
        for event in crate::mpv::sfx::take_completions() {
            match event {
                Completion::Imported(effect) => {
                    if crate::mpv::sfx::previewing(effect.id) {
                        let _ = crate::mpv::sfx::stop_preview();
                    }
                    let mut config = crate::platform::interop::get_live_config();
                    config.audio_effects.retain(|e| e.id != effect.id);
                    config.audio_effects.push(effect.clone());
                    if let Err(error) = config.validate() {
                        self.set_osd(error);
                        continue;
                    }
                    crate::platform::interop::set_live_config(config);
                    for template in &mut self.timeline.templates {
                        if template
                            .audio_effect
                            .as_ref()
                            .is_some_and(|e| e.id == effect.id)
                        {
                            template.name.clone_from(&effect.name);
                            template.icon.clone_from(&effect.icon);
                            template.duration_ms = effect.duration_ms;
                            template.audio_effect = Some(effect.clone());
                        }
                    }
                    if self.effect_library_draft.id == effect.id.to_string() {
                        self.select_audio_effect(effect.id);
                    }
                    self.hardware_effect_authoring.status = "Audio effect saved".into();
                    self.sync_timeline_engine();
                    self.save_config();
                    self.last_web_broadcast = None;
                }
                Completion::Error(error) => {
                    let _ = crate::messaging::publish(
                        crate::messaging::ToastRequest {
                            id: Some("sfx.error".into()),
                            title: "Audio effect".into(),
                            message: error.clone(),
                            severity: crate::messaging::Severity::Error,
                            timeout_ms: 8000,
                        },
                        "audio",
                    );
                    self.hardware_effect_authoring.status = error;
                }
            }
        }
    }
    pub(crate) fn delete_audio_effect(&mut self, id: uuid::Uuid) {
        let mut config = crate::platform::interop::get_live_config();
        config.audio_effects.retain(|e| e.id != id);
        crate::platform::interop::set_live_config(config);
        let removed = self
            .timeline
            .templates
            .iter()
            .filter(|t| t.audio_effect.as_ref().is_some_and(|e| e.id == id))
            .map(|t| t.id)
            .collect::<std::collections::HashSet<_>>();
        self.timeline
            .instances
            .retain(|i| !removed.contains(&i.effect_id));
        self.timeline.templates.retain(|t| !removed.contains(&t.id));
        let _ = crate::mpv::sfx::stop_preview();
        self.sync_timeline_engine();
        self.save_config();
        self.last_web_broadcast = None;
    }
}
