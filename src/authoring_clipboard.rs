//! Cue interchange preserves exact times, templates, values and references.
//! Copy is inert; Cut mutates only after the OS accepted the clipboard write.
use crate::{app::PealayerApp, four_d::models::Timeline};
use eframe::egui;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cues {
    format: String,
    timeline: Timeline,
}
pub fn encode(
    timeline: &Timeline,
    selected: &std::collections::HashSet<uuid::Uuid>,
) -> Result<String, String> {
    let mut fragment = Timeline::default();
    fragment.instances = timeline
        .instances
        .iter()
        .filter(|i| selected.contains(&i.id))
        .cloned()
        .collect();
    if fragment.instances.is_empty() {
        return Err("Select cues to copy".into());
    }
    let base = fragment
        .instances
        .iter()
        .map(|i| i.start_time_ms)
        .min()
        .unwrap();
    fragment.templates = timeline
        .templates
        .iter()
        .filter(|t| fragment.instances.iter().any(|i| i.effect_id == t.id))
        .cloned()
        .collect();
    for cue in &mut fragment.instances {
        cue.start_time_ms -= base;
    }
    crate::projects::TimelineDocument {
        timeline: fragment.clone(),
        view: Default::default(),
    }
    .validate()?;
    serde_json::to_string(&Cues {
        format: "pealayer.timeline-cues".into(),
        timeline: fragment,
    })
    .map_err(|e| e.to_string())
}
pub fn decode(text: &str) -> Result<Timeline, String> {
    if text.len() > crate::platform::clipboard::MAX_BYTES {
        return Err("Clipboard selection exceeds 8 MiB".into());
    }
    let cues: Cues =
        serde_json::from_str(text).map_err(|_| "Clipboard does not contain timeline cues")?;
    if cues.format != "pealayer.timeline-cues" || cues.timeline.instances.is_empty() {
        return Err("Clipboard does not contain timeline cues".into());
    }
    crate::projects::TimelineDocument {
        timeline: cues.timeline.clone(),
        view: Default::default(),
    }
    .validate()?;
    Ok(cues.timeline)
}
impl PealayerApp {
    pub(crate) fn copy_timeline_cues(&mut self, cut: bool) -> Result<(), String> {
        self.copy_timeline_cues_using(cut, crate::platform::clipboard::write)
    }
    fn copy_timeline_cues_using(
        &mut self,
        cut: bool,
        write: impl FnOnce(&str) -> Result<(), String>,
    ) -> Result<(), String> {
        if cut && (!self.is_paused || crate::peer::active()) {
            return Err("Pause local playback before cutting cues".into());
        }
        let text = encode(&self.timeline, &self.selected_instance_ids)?;
        // The same lock validation as Paste, but on an untouched candidate.
        if cut {
            self.validate_asset_fragment_locks(&decode(&text)?)?;
        }
        write(&text)?;
        if cut {
            self.delete_selected_timeline_cues();
        }
        Ok(())
    }
    pub(crate) fn paste_timeline_cues(&mut self, text: &str) -> Result<(), String> {
        if crate::peer::active() {
            return Err("Paste cues in the authority's timeline".into());
        }
        let fragment = decode(text)?;
        self.append_asset_fragment(
            &fragment,
            (self.playback_time.max(0.0) * 1000.0).round() as u64,
        )
    }
    pub(crate) fn process_clipboard_shortcuts(&mut self, ctx: &egui::Context, timeline: bool) {
        if ctx.text_edit_focused()
            || egui::Popup::is_any_open(ctx)
            || ctx.memory(|m| m.top_modal_layer().is_some())
            || self.show_effect_library_editor
            || self.show_preferences_dialog
        {
            return;
        }
        let (copy, cut, paste, event) = ctx.input(|i| {
            let command = i.modifiers.command || i.modifiers.ctrl;
            (
                command && i.key_pressed(egui::Key::C)
                    || i.events.iter().any(|e| matches!(e, egui::Event::Copy)),
                command && i.key_pressed(egui::Key::X)
                    || i.events.iter().any(|e| matches!(e, egui::Event::Cut)),
                command && i.key_pressed(egui::Key::V)
                    || i.events.iter().any(|e| matches!(e, egui::Event::Paste(_))),
                i.events.iter().find_map(|e| {
                    if let egui::Event::Paste(t) = e {
                        Some(t.clone())
                    } else {
                        None
                    }
                }),
            )
        });
        if !(paste || timeline && (copy || cut)) {
            return;
        }
        ctx.input_mut(|i| {
            i.events.retain(|e| {
                !matches!(
                    e,
                    egui::Event::Copy | egui::Event::Cut | egui::Event::Paste(_)
                )
            });
        });
        let result = if timeline && (copy || cut) {
            self.copy_timeline_cues(cut)
        } else {
            (|| {
                let destination = if timeline {
                    crate::assets::Destination::Timeline {
                        start_time_ms: (self.playback_time.max(0.0) * 1000.0).round() as u64,
                        track_key: self.selected_timeline_track.clone(),
                    }
                } else {
                    crate::assets::native_destination(ctx)
                };
                let files = crate::platform::clipboard::files()?;
                if !files.is_empty() {
                    return self.import_assets(
                        crate::assets::ImportRequest {
                            paths: files,
                            destination,
                            owner_id: None,
                        },
                        ctx,
                    );
                }
                let text = event
                    .map(Ok)
                    .unwrap_or_else(crate::platform::clipboard::text)?;
                if timeline && decode(&text).is_ok() {
                    return self.paste_timeline_cues(&text);
                }
                if let Some(paths) = crate::platform::clipboard::text_paths(&text) {
                    return self.import_assets(
                        crate::assets::ImportRequest {
                            paths,
                            destination,
                            owner_id: None,
                        },
                        ctx,
                    );
                }
                if let Some(url) = crate::platform::clipboard_urls::candidate(&text) {
                    return crate::remote_location::request(&url, None, true, ctx);
                }
                Err(
                    "Paste a movie, subtitle, sound effect, asset document, URL or copied cues"
                        .into(),
                )
            })()
        };
        if let Err(error) = result {
            self.set_osd(error);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::four_d::models::{Effect, EffectInstance};
    fn fixture() -> (Timeline, std::collections::HashSet<uuid::Uuid>) {
        let mut timeline = Timeline::default();
        let effect = Effect::new("Exact effect".into(), "lightning".into(), 1234, Vec::new());
        let one = EffectInstance {
            id: uuid::Uuid::new_v4(),
            effect_id: effect.id,
            start_time_ms: 10001,
        };
        let two = EffectInstance {
            id: uuid::Uuid::new_v4(),
            effect_id: effect.id,
            start_time_ms: 10777,
        };
        let selection = [one.id, two.id].into_iter().collect();
        timeline.templates.push(effect);
        timeline.instances.extend([one, two]);
        (timeline, selection)
    }
    #[test]
    fn cue_copy_preserves_values_relative_timing_and_rekeys_on_paste() {
        let (source, selected) = fixture();
        let copy = decode(&encode(&source, &selected).unwrap()).unwrap();
        assert_eq!(
            copy.instances
                .iter()
                .map(|i| i.start_time_ms)
                .collect::<Vec<_>>(),
            [0, 776]
        );
        assert_eq!(copy.templates, source.templates);
        let mut target = Timeline::default();
        let ids = crate::assets::append_timeline(&mut target, &copy, 20003).unwrap();
        assert_eq!(
            target
                .instances
                .iter()
                .map(|i| i.start_time_ms)
                .collect::<Vec<_>>(),
            [20003, 20779]
        );
        assert!(ids.iter().all(|id| !selected.contains(id)));
        assert_ne!(target.templates[0].id, source.templates[0].id);
        assert_eq!(target.templates[0].duration_ms, 1234);
    }
    #[test]
    fn malformed_or_oversized_clipboard_is_rejected_without_mutation() {
        let (source, selected) = fixture();
        let mut value: serde_json::Value =
            serde_json::from_str(&encode(&source, &selected).unwrap()).unwrap();
        value["timeline"]["instances"][0]["effect_id"] = uuid::Uuid::new_v4().to_string().into();
        assert!(decode(&value.to_string()).is_err());
        assert!(decode(&" ".repeat(crate::platform::clipboard::MAX_BYTES + 1)).is_err());
        assert!(encode(&source, &Default::default()).is_err());
    }
    #[test]
    fn paste_is_one_undoable_edit_and_does_not_seek() {
        let _lock = crate::app::tests::lock_app_tests();
        let mut app = PealayerApp::default();
        app.is_paused = true;
        app.playback_time = 14.321;
        let (source, selected) = fixture();
        let text = encode(&source, &selected).unwrap();
        let before = app.timeline.clone();
        let undo = app.undo_stack.undo_len();
        app.paste_timeline_cues(&text).unwrap();
        assert_eq!(app.playback_time, 14.321);
        assert_eq!(app.undo_stack.undo_len(), undo + 1);
        assert_eq!(app.timeline.instances[0].start_time_ms, 14321);
        assert_eq!(app.selected_instance_ids.len(), 2);
        let current = app.snapshot_timeline();
        assert_eq!(
            app.undo_stack.undo(current).unwrap().instances,
            before.instances
        );
    }
    #[test]
    fn cut_is_atomic_when_clipboard_is_busy_and_respects_locked_outputs() {
        let _lock = crate::app::tests::lock_app_tests();
        let mut app = PealayerApp::default();
        app.is_paused = true;
        let (timeline, selected) = fixture();
        app.timeline = timeline;
        app.selected_instance_ids = selected;
        let before = app.timeline.clone();
        assert!(
            app.copy_timeline_cues_using(true, |_| Err("busy".into()))
                .is_err()
        );
        assert_eq!(app.timeline, before);
        app.timeline.templates[0]
            .actions
            .push(crate::four_d::models::AtomicAction {
                relay_id: 5,
                state: true,
                offset_ms: 0,
            });
        app.track_locked.insert(5);
        assert!(
            app.copy_timeline_cues_using(true, |_| panic!("locked cuts must not write clipboard"))
                .is_err()
        );
        assert_eq!(app.timeline.instances, before.instances);
        app.track_locked.clear();
        app.copy_timeline_cues_using(true, |_| Ok(())).unwrap();
        assert!(app.timeline.instances.is_empty());
        assert_eq!(app.undo_stack.undo_len(), 1);
    }
}
