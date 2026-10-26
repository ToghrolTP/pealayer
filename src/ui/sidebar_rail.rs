use std::ops::RangeInclusive;
use eframe::egui::load::Bytes;
use eframe::egui::{Id, ImageSource, Ui};
use craft_ui::panes::{
    Edge, PaneContent, PaneHost, PaneOutput, PaneRequest, PaneState, Rail, RailItem,
    RailRequest, SidePolicy,
};
use crate::app::PealayerApp;
use crate::ui::layout::PealayerTab;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SidebarPane {
    HardwareMonitor,
    MediaInspector,
    EffectControls,
    EffectsLibrary,
}

impl SidebarPane {
    pub const ALL: [SidebarPane; 4] = [
        SidebarPane::HardwareMonitor,
        SidebarPane::MediaInspector,
        SidebarPane::EffectControls,
        SidebarPane::EffectsLibrary,
    ];

    pub fn title(self, language: crate::config::AppLanguage) -> String {
        PealayerTab::from(self).title_for_language(language)
    }
}

impl From<SidebarPane> for PealayerTab {
    fn from(pane: SidebarPane) -> Self {
        match pane {
            SidebarPane::HardwareMonitor => PealayerTab::HardwareMonitor,
            SidebarPane::MediaInspector => PealayerTab::MediaInspector,
            SidebarPane::EffectControls => PealayerTab::EffectControls,
            SidebarPane::EffectsLibrary => PealayerTab::EffectsLibrary,
        }
    }
}

impl TryFrom<PealayerTab> for SidebarPane {
    type Error = ();
    fn try_from(tab: PealayerTab) -> Result<Self, Self::Error> {
        match tab {
            PealayerTab::HardwareMonitor => Ok(SidebarPane::HardwareMonitor),
            PealayerTab::MediaInspector => Ok(SidebarPane::MediaInspector),
            PealayerTab::EffectControls => Ok(SidebarPane::EffectControls),
            PealayerTab::EffectsLibrary => Ok(SidebarPane::EffectsLibrary),
            _ => Err(()),
        }
    }
}

// Embedded clean 24x24 SVG vectors for high-density rail rendering
const SVG_CIRCUITRY: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="18" x="3" y="3" rx="2"/><path d="M15 9h.01"/><path d="M9 9h.01"/><path d="M9 15h.01"/><path d="M15 15h.01"/><path d="M9 12h6"/><path d="M12 9v6"/></svg>"##;
const SVG_INFO: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 16v-4"/><path d="M12 8h.01"/></svg>"##;
const SVG_SLIDERS: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="4" x2="4" y1="21" y2="14"/><line x1="4" x2="4" y1="10" y2="3"/><line x1="12" x2="12" y1="21" y2="12"/><line x1="12" x2="12" y1="8" y2="3"/><line x1="20" x2="20" y1="21" y2="16"/><line x1="20" x2="20" y1="12" y2="3"/><line x1="1" x2="7" y1="14" y2="14"/><line x1="9" x2="15" y1="8" y2="8"/><line x1="17" x2="23" y1="16" y2="16"/></svg>"##;
const SVG_SPARKLE: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m12 3-1.912 5.813a2 2 0 0 1-1.275 1.275L3 12l5.813 1.912a2 2 0 0 1 1.275 1.275L12 21l1.912-5.813a2 2 0 0 1 1.275-1.275L21 12l-5.813-1.912a2 2 0 0 1-1.275-1.275L12 3Z"/></svg>"##;

pub fn icon_source(pane: SidebarPane) -> ImageSource<'static> {
    let (uri, bytes) = match pane {
        SidebarPane::HardwareMonitor => ("bytes://pealayer/sidebar/hardware.svg", SVG_CIRCUITRY),
        SidebarPane::MediaInspector => ("bytes://pealayer/sidebar/inspector.svg", SVG_INFO),
        SidebarPane::EffectControls => ("bytes://pealayer/sidebar/controls.svg", SVG_SLIDERS),
        SidebarPane::EffectsLibrary => ("bytes://pealayer/sidebar/library.svg", SVG_SPARKLE),
    };
    ImageSource::Bytes {
        uri: uri.into(),
        bytes: Bytes::Static(bytes),
    }
}

pub struct SidebarPaneContent<'a> {
    pub app: &'a mut PealayerApp,
}

impl<'a> SidebarPaneContent<'a> {
    pub const DEFAULT_WIDTH: f32 = 300.0;
    pub const MIN_WIDTH: f32 = 240.0;
    pub const MAX_WIDTH: f32 = 600.0;

    pub fn default_pane_state(_pane: &SidebarPane) -> PaneState {
        PaneState::docked(Edge::End, Self::DEFAULT_WIDTH)
    }

    pub fn pane_size_range(_pane: &SidebarPane) -> RangeInclusive<f32> {
        Self::MIN_WIDTH..=Self::MAX_WIDTH
    }

    pub fn pane_movable(_pane: &SidebarPane) -> bool {
        false
    }

    pub fn pane_closable(_pane: &SidebarPane) -> bool {
        true
    }
}

impl<'a> PaneContent<SidebarPane> for SidebarPaneContent<'a> {
    fn title(&self, pane: &SidebarPane) -> String {
        pane.title(self.app.language)
    }

    fn body(&mut self, ui: &mut Ui, pane: &SidebarPane) {
        let mut tab = PealayerTab::from(*pane);
        let mut viewer = crate::ui::layout::PealayerTabViewer { app: self.app };
        egui_dock::TabViewer::ui(&mut viewer, ui, &mut tab);
    }

    fn default_state(&self, pane: &SidebarPane) -> PaneState {
        Self::default_pane_state(pane)
    }

    fn size_range(&self, pane: &SidebarPane) -> RangeInclusive<f32> {
        Self::pane_size_range(pane)
    }

    fn movable(&self, pane: &SidebarPane) -> bool {
        Self::pane_movable(pane)
    }

    fn closable(&self, pane: &SidebarPane) -> bool {
        Self::pane_closable(pane)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SidebarRailState {
    pub open: Option<SidebarPane>,
    pub width: f32,
    pub enabled: bool,
}

impl Default for SidebarRailState {
    fn default() -> Self {
        Self {
            open: None,
            width: SidebarPaneContent::DEFAULT_WIDTH,
            enabled: true,
        }
    }
}

impl SidebarRailState {
    pub fn apply_rail_request(&mut self, request: RailRequest<SidebarPane>) {
        match request {
            RailRequest::Open(pane) => self.open = Some(pane),
            RailRequest::Close(_) => self.open = None,
            _ => {}
        }
    }

    pub fn apply_pane_request(&mut self, request: PaneRequest<SidebarPane>) {
        match request {
            PaneRequest::Close(_) => self.open = None,
            PaneRequest::Resize(_, size) => {
                self.width = size.clamp(SidebarPaneContent::MIN_WIDTH, SidebarPaneContent::MAX_WIDTH)
            }
            _ => {}
        }
    }
}

pub fn show_sidebar_rail(
    app: &mut PealayerApp,
    ui: &mut Ui,
    state: &mut SidebarRailState,
) -> (Option<RailRequest<SidebarPane>>, Option<PaneOutput<SidebarPane>>) {
    if !state.enabled {
        return (None, None);
    }

    let items: Vec<RailItem<SidebarPane>> = SidebarPane::ALL
        .iter()
        .map(|pane| {
            RailItem::new(*pane, icon_source(*pane), pane.title(app.language))
        })
        .collect();

    let rail_out = Rail::new(Id::unique("pealayer-sidebar-rail"))
        .edge(Edge::End)
        .rtl(app.rtl)
        .width(crate::ui::craft_theme::RAIL_WIDTH)
        .button_size(crate::ui::craft_theme::RAIL_BUTTON)
        .show(ui, &items, state.open.as_ref(), |_| {});

    if let Some(ref req) = rail_out.request {
        state.apply_rail_request(req.clone());
    }

    let pane_out = if let Some(pane) = state.open {
        let open_panes = [pane];
        let mut content = SidebarPaneContent { app };
        let out = PaneHost::new(Id::unique("pealayer-sidebar-pane-host"))
            .rtl(content.app.rtl)
            .policy(SidePolicy::OneSlot)
            .show(
                ui,
                &open_panes,
                |_| Some(PaneState::docked(Edge::End, state.width)),
                &mut content,
            );
        for req in &out.requests {
            state.apply_pane_request(req.clone());
        }
        Some(out)
    } else {
        None
    };

    (rail_out.request, pane_out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use craft_ui::panes::{Edge, PaneContent, PaneRequest, Place, RailRequest};

    #[test]
    fn sidebar_rail_state_request_updates() {
        let mut state = SidebarRailState::default();
        assert_eq!(state.open, None);
        assert_eq!(state.width, 300.0);
        assert!(state.enabled);

        state.apply_rail_request(RailRequest::Open(SidebarPane::HardwareMonitor));
        assert_eq!(state.open, Some(SidebarPane::HardwareMonitor));

        state.apply_rail_request(RailRequest::Close(SidebarPane::HardwareMonitor));
        assert_eq!(state.open, None);

        state.apply_pane_request(PaneRequest::Resize(SidebarPane::HardwareMonitor, 360.0));
        assert_eq!(state.width, 360.0);

        state.open = Some(SidebarPane::MediaInspector);
        state.apply_pane_request(PaneRequest::Close(SidebarPane::MediaInspector));
        assert_eq!(state.open, None);
    }

    struct SidebarPaneContentMock;

    impl PaneContent<SidebarPane> for SidebarPaneContentMock {
        fn title(&self, pane: &SidebarPane) -> String {
            pane.title(crate::config::AppLanguage::English)
        }

        fn body(&mut self, _ui: &mut Ui, _pane: &SidebarPane) {}

        fn default_state(&self, pane: &SidebarPane) -> PaneState {
            SidebarPaneContent::default_pane_state(pane)
        }

        fn size_range(&self, pane: &SidebarPane) -> RangeInclusive<f32> {
            SidebarPaneContent::pane_size_range(pane)
        }

        fn movable(&self, pane: &SidebarPane) -> bool {
            SidebarPaneContent::pane_movable(pane)
        }

        fn closable(&self, pane: &SidebarPane) -> bool {
            SidebarPaneContent::pane_closable(pane)
        }
    }

    #[test]
    fn sidebar_pane_conversions_and_titles() {
        assert_eq!(SidebarPane::from(SidebarPane::HardwareMonitor), SidebarPane::HardwareMonitor);
        let tab: crate::ui::layout::PealayerTab = SidebarPane::EffectsLibrary.into();
        assert_eq!(tab, crate::ui::layout::PealayerTab::EffectsLibrary);
        assert_eq!(SidebarPane::try_from(crate::ui::layout::PealayerTab::Timeline), Err(()));
        assert_eq!(SidebarPane::try_from(crate::ui::layout::PealayerTab::MediaInspector), Ok(SidebarPane::MediaInspector));
    }

    #[test]
    fn sidebar_pane_icons_are_valid_svg_sources() {
        for pane in SidebarPane::ALL {
            let source = icon_source(pane);
            match source {
                ImageSource::Bytes { uri, bytes } => {
                    assert!(uri.starts_with("bytes://pealayer/sidebar/"));
                    assert!(!bytes.is_empty());
                }
                _ => panic!("Expected ImageSource::Bytes for {pane:?}"),
            }
        }
    }

    #[test]
    fn sidebar_pane_content_defaults_match_craft_spec() {
        let content = SidebarPaneContentMock;
        for pane in SidebarPane::ALL {
            let state = content.default_state(&pane);
            assert_eq!(state.place, Place::Docked(Edge::End));
            assert_eq!(state.size, Some(300.0));
            assert!(!content.movable(&pane));
            assert!(content.closable(&pane));
            let range = content.size_range(&pane);
            assert_eq!(*range.start(), 240.0);
            assert_eq!(*range.end(), 600.0);
        }
    }

    #[test]
    fn sidebar_pane_content_with_app_matches_craft_spec() {
        let mut app = PealayerApp::default();
        let content = SidebarPaneContent { app: &mut app };
        for pane in SidebarPane::ALL {
            let state = content.default_state(&pane);
            assert_eq!(state.place, Place::Docked(Edge::End));
            assert_eq!(state.size, Some(300.0));
            assert!(!content.movable(&pane));
            assert!(content.closable(&pane));
            let range = content.size_range(&pane);
            assert_eq!(*range.start(), 240.0);
            assert_eq!(*range.end(), 600.0);
            assert!(!content.title(&pane).is_empty());
        }
    }

    fn discard_ui_output(mut output: eframe::egui::FullOutput) {
        output.textures_delta.clear();
    }

    #[test]
    fn show_sidebar_rail_disabled_returns_none() {
        let mut app = PealayerApp::default();
        let mut state = SidebarRailState {
            open: Some(SidebarPane::HardwareMonitor),
            width: 320.0,
            enabled: false,
        };
        let ctx = eframe::egui::Context::default();
        discard_ui_output(ctx.run_ui(eframe::egui::RawInput::default(), |ui| {
            let (rail_req, pane_out) = show_sidebar_rail(&mut app, ui, &mut state);
            assert!(rail_req.is_none());
            assert!(pane_out.is_none());
        }));
    }

    #[test]
    fn show_sidebar_rail_renders_when_enabled() {
        let mut app = PealayerApp::default();
        let mut state = SidebarRailState::default();
        let ctx = eframe::egui::Context::default();
        discard_ui_output(ctx.run_ui(eframe::egui::RawInput::default(), |ui| {
            let (rail_req, pane_out) = show_sidebar_rail(&mut app, ui, &mut state);
            assert!(rail_req.is_none());
            assert!(pane_out.is_none());
        }));

        state.open = Some(SidebarPane::HardwareMonitor);
        discard_ui_output(ctx.run_ui(eframe::egui::RawInput::default(), |ui| {
            let (rail_req, pane_out) = show_sidebar_rail(&mut app, ui, &mut state);
            assert!(rail_req.is_none());
            assert!(pane_out.is_some());
        }));
    }
}
