//! Persisted, surface-scoped pointer policy. No output or persistence on lookup.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    #[default]
    None,
    Inherit,
    SeekBackward,
    SeekForward,
    PreviousChapter,
    NextChapter,
    PreviousFile,
    NextFile,
    FrameBackward,
    FrameForward,
    PlayPause,
    ToggleMute,
    ToggleFullscreen,
    BossKey,
    PanLeft,
    PanRight,
    ZoomOut,
    ZoomIn,
    HistoryBack,
    HistoryForward,
    ParentFolder,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Buttons {
    pub back: Action,
    pub forward: Action,
    pub ctrl_back: Action,
    pub ctrl_forward: Action,
    pub alt_back: Action,
    pub alt_forward: Action,
    pub shift_back: Action,
    pub shift_forward: Action,
}
impl Default for Buttons {
    fn default() -> Self {
        Self {
            back: Action::None,
            forward: Action::None,
            ctrl_back: Action::Inherit,
            ctrl_forward: Action::Inherit,
            alt_back: Action::Inherit,
            alt_forward: Action::Inherit,
            shift_back: Action::Inherit,
            shift_forward: Action::Inherit,
        }
    }
}
impl Buttons {
    /// Match at mouse-down. Combined modifiers use Ctrl/Command, Alt, Shift.
    pub fn resolve(self, forward: bool, ctrl: bool, alt: bool, shift: bool) -> Action {
        let base = if forward { self.forward } else { self.back };
        let action = if ctrl {
            if forward {
                self.ctrl_forward
            } else {
                self.ctrl_back
            }
        } else if alt {
            if forward {
                self.alt_forward
            } else {
                self.alt_back
            }
        } else if shift {
            if forward {
                self.shift_forward
            } else {
                self.shift_back
            }
        } else {
            base
        };
        if action == Action::Inherit {
            base
        } else {
            action
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Bindings {
    pub video: Buttons,
    pub timeline: Buttons,
    pub effect_designer: Buttons,
    pub remote_browser: Buttons,
    pub ctrl_left_drag: crate::config::PlayerDragAction,
    pub alt_middle_drag: crate::config::PlayerDragAction,
    pub shift_right_context_menu: bool,
}
impl Default for Bindings {
    fn default() -> Self {
        use Action::*;
        Self {
            video: Buttons {
                back: SeekBackward,
                forward: SeekForward,
                ctrl_back: PreviousChapter,
                ctrl_forward: NextChapter,
                alt_back: FrameBackward,
                alt_forward: FrameForward,
                shift_back: PreviousFile,
                shift_forward: NextFile,
                ..Default::default()
            },
            timeline: Buttons {
                back: PanLeft,
                forward: PanRight,
                ctrl_back: ZoomOut,
                ctrl_forward: ZoomIn,
                ..Default::default()
            },
            effect_designer: Buttons {
                back: PanLeft,
                forward: PanRight,
                ctrl_back: ZoomOut,
                ctrl_forward: ZoomIn,
                ..Default::default()
            },
            remote_browser: Buttons {
                back: HistoryBack,
                forward: HistoryForward,
                ctrl_back: ParentFolder,
                ..Default::default()
            },
            ctrl_left_drag: crate::config::PlayerDragAction::MoveWindow,
            alt_middle_drag: crate::config::PlayerDragAction::Seek,
            shift_right_context_menu: true,
        }
    }
}
impl Bindings {
    pub fn validate(&self) -> Result<(), String> {
        use Action::*;
        for (scope, buttons) in [
            ("video", self.video),
            ("timeline", self.timeline),
            ("effect_designer", self.effect_designer),
            ("remote_browser", self.remote_browser),
        ] {
            if [buttons.back, buttons.forward].contains(&Inherit) {
                return Err(format!(
                    "mouse_bindings.{scope}: unmodified actions cannot inherit"
                ));
            }
            for action in [
                buttons.back,
                buttons.forward,
                buttons.ctrl_back,
                buttons.ctrl_forward,
                buttons.alt_back,
                buttons.alt_forward,
                buttons.shift_back,
                buttons.shift_forward,
            ] {
                let valid = matches!(action, None | Inherit)
                    || match scope {
                        "video" => matches!(
                            action,
                            SeekBackward
                                | SeekForward
                                | PreviousChapter
                                | NextChapter
                                | PreviousFile
                                | NextFile
                                | FrameBackward
                                | FrameForward
                                | PlayPause
                                | ToggleMute
                                | ToggleFullscreen
                                | BossKey
                        ),
                        "timeline" | "effect_designer" => {
                            matches!(action, PanLeft | PanRight | ZoomOut | ZoomIn)
                        }
                        _ => matches!(action, HistoryBack | HistoryForward | ParentFolder),
                    };
                if !valid {
                    return Err(format!(
                        "mouse_bindings.{scope}: action is not supported on this surface"
                    ));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn modifiers_and_scopes_have_deterministic_independent_defaults() {
        let b = Bindings::default();
        assert!(b.validate().is_ok());
        assert_eq!(
            b.video.resolve(false, false, false, false),
            Action::SeekBackward
        );
        assert_eq!(
            b.timeline.resolve(false, true, false, false),
            Action::ZoomOut
        );
        assert_eq!(b.video.resolve(true, true, true, true), Action::NextChapter);
        assert_eq!(
            b.timeline.resolve(true, false, true, false),
            Action::PanRight
        );
        assert_eq!(
            b.remote_browser.resolve(false, true, false, false),
            Action::ParentFolder
        );
        let mut custom = b.clone();
        custom.video.back = Action::BossKey;
        let restored: Bindings =
            serde_json::from_value(serde_json::to_value(&custom).unwrap()).unwrap();
        assert_eq!(restored, custom);
        assert_eq!(restored.timeline, b.timeline);
        custom.timeline.back = Action::BossKey;
        assert!(custom.validate().is_err());
    }
}
