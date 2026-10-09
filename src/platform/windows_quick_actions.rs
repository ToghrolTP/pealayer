//! Shared by the resource compiler and Jump List publisher. The application
//! logo remains resource 1; task icons have stable, explicit resource IDs.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowsQuickAction {
    pub title: &'static str,
    pub arguments: &'static str,
    pub icon_resource_id: u16,
    pub icon_glyph: &'static str,
}

impl WindowsQuickAction {
    pub const fn icon_location_index(self) -> i32 {
        // A negative shell icon index selects an explicit RT_GROUP_ICON ID,
        // independent of resource ordering or custom application branding.
        -(self.icon_resource_id as i32)
    }
}

pub const WINDOWS_QUICK_ACTIONS: [WindowsQuickAction; 7] = [
    WindowsQuickAction {
        title: "Play / Pause",
        arguments: "--toggle-pause",
        icon_resource_id: 101,
        icon_glyph: egui_phosphor::regular::PLAY_PAUSE,
    },
    WindowsQuickAction {
        title: "Previous chapter",
        arguments: "--chapter-previous",
        icon_resource_id: 102,
        icon_glyph: egui_phosphor::regular::SKIP_BACK,
    },
    WindowsQuickAction {
        title: "Next chapter",
        arguments: "--chapter-next",
        icon_resource_id: 103,
        icon_glyph: egui_phosphor::regular::SKIP_FORWARD,
    },
    WindowsQuickAction {
        title: "Mute / Unmute",
        arguments: "--toggle-mute",
        icon_resource_id: 104,
        icon_glyph: egui_phosphor::regular::SPEAKER_SLASH,
    },
    WindowsQuickAction {
        title: "Toggle fullscreen",
        arguments: "--toggle-fullscreen",
        icon_resource_id: 105,
        icon_glyph: egui_phosphor::regular::ARROWS_OUT,
    },
    WindowsQuickAction {
        title: "Preferences",
        arguments: "--preferences",
        icon_resource_id: 106,
        icon_glyph: egui_phosphor::regular::GEAR,
    },
    WindowsQuickAction {
        title: "Exit Pealayer",
        arguments: "--quit",
        icon_resource_id: 107,
        icon_glyph: egui_phosphor::regular::SIGN_OUT,
    },
];
