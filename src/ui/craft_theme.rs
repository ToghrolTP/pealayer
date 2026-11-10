//! Craft UI & FilmCraft Design Tokens and Theme System.
//!
//! Every widget and panel reads colors, spacing, and metrics from [`Tokens`].
//! The palette and density are seamless to FilmCraft (Premiere-style professional dark UI),
//! powered by `craft-ui::theme::WidgetPalette`.

use eframe::egui::{
    self, Color32, CornerRadius, FontId, FontFamily, Margin, Stroke, StrokeKind, TextStyle, Vec2, Visuals,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeKind {
    /// Premiere-style darkest (default).
    #[default]
    Dark,
    /// Slightly lighter grey panels (Premiere's brightness slider mid position).
    Medium,
    /// Clean high-contrast light theme.
    Light,
}

impl ThemeKind {
    pub fn pref_name(self) -> &'static str {
        match self {
            ThemeKind::Dark => "darkest",
            ThemeKind::Medium => "dark",
            ThemeKind::Light => "light",
        }
    }

    pub fn from_pref(s: &str) -> ThemeKind {
        match s {
            "dark" => ThemeKind::Medium,
            "light" => ThemeKind::Light,
            _ => ThemeKind::Dark,
        }
    }

    pub fn from_name(s: &str) -> Option<ThemeKind> {
        match s.to_ascii_lowercase().as_str() {
            "dark" | "darkest" => Some(ThemeKind::Dark),
            "medium" | "grey" | "gray" => Some(ThemeKind::Medium),
            "light" => Some(ThemeKind::Light),
            _ => None,
        }
    }
}

/// Full set of design tokens seamless to FilmCraft.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tokens {
    pub kind: ThemeKind,
    /// Space between panels / outer app background.
    pub app_bg: Color32,
    /// Header bar (top).
    pub header_bg: Color32,
    /// Panel body.
    pub panel_bg: Color32,
    /// Panel tab strip.
    pub tab_bg: Color32,
    pub tab_text: Color32,
    pub tab_text_active: Color32,
    /// Blue focus rectangle around the active/focused panel.
    pub focus: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub text_faint: Color32,
    pub icon: Color32,
    pub icon_active: Color32,
    pub hover: Color32,
    pub pressed: Color32,
    pub field_bg: Color32,
    pub field_border: Color32,
    pub separator: Color32,
    /// Well background (recessed status bar, card container, search well).
    pub well_bg: Color32,
    /// Card background surface.
    pub card_bg: Color32,
    /// Subtle border stroke for modals, cards, and dividers.
    pub border_subtle: Color32,
    /// Modal overlay backdrop dimming fill.
    pub modal_backdrop: Color32,
    pub row_alt: Color32,
    pub row_selected: Color32,
    /// Scrubby (hot-text) numeric value color.
    pub hot_text: Color32,
    /// Selected pills and compact toggle backgrounds.
    pub pill_active_bg: Color32,
    /// The selected clip bar in Effect Controls.
    pub clip_bar_bg: Color32,
    /// Raised audio / hardware control surfaces.
    pub control_surface: Color32,
    /// Recessed audio / hardware control tracks and panner wells.
    pub control_well: Color32,
    pub control_border: Color32,
    pub control_handle_bg: Color32,
    pub control_handle: Color32,
    pub control_handle_dim: Color32,
    /// Data-plot surfaces (keyframe, EQ, dynamics graphs), distinct from media canvases.
    pub plot_bg: Color32,
    pub plot_grid: Color32,
    pub plot_axis: Color32,
    pub plot_handle: Color32,
    pub plot_handle_dim: Color32,
    /// Meter wells and separate loudness readout surface.
    pub meter_bg: Color32,
    pub meter_readout_bg: Color32,
    pub meter_clip_off: Color32,
    pub panner_bg: Color32,
    pub panner_border: Color32,
    pub panner_speaker: Color32,
    pub fader_knob: Color32,
    pub eq_track: Color32,
    pub eq_tick: Color32,
    pub eq_knob: Color32,
    pub eq_knob_active: Color32,
    pub eq_node: Color32,
    pub eq_node_off: Color32,
    pub crossover_handle: Color32,
    pub keyframe_plot_bg: Color32,
    pub keyframe_plot_grid: Color32,
    pub keyframe_plot_axis: Color32,
    pub keyframe_handle: Color32,
    pub switch_fill: Color32,
    pub switch_knob: Color32,
    pub switch_border: Color32,
    pub slider_disabled_track: Color32,
    pub slider_disabled_knob: Color32,
    pub button_hover: Color32,
    pub button_border: Color32,
    pub caption_track_bg: Color32,
    pub scroll_thumb: Color32,
    pub scroll_thumb_hover: Color32,
    pub vertical_scroll_thumb: Color32,
    pub disabled_clip_bg: Color32,
    // Timeline tokens
    pub tl_bg: Color32,
    pub tl_track_bg: Color32,
    pub tl_track_bg_alt: Color32,
    pub tl_header_bg: Color32,
    pub tl_ruler_bg: Color32,
    pub tl_ruler_tick: Color32,
    pub tl_ruler_text: Color32,
    pub playhead: Color32,
    /// Fill of a track header's source-patch / track-targeting button while active.
    pub track_target: Color32,
    pub in_out_shade: Color32,
    pub clip_selected_border: Color32,
    pub render_red: Color32,
    pub render_yellow: Color32,
    pub render_green: Color32,
    pub monitor_bg: Color32,
    pub timecode: Color32,
    pub danger: Color32,
    /// Razor-sharp 0.0 panel corner radius (FilmCraft standard).
    pub radius: f32,
    /// Subtle 4.0 widget corner radius (FilmCraft standard).
    pub radius_sm: f32,
    pub gap: f32,
    pub tab_h: f32,
}

impl Tokens {
    /// Craft supplies geometry and semantic roles, not a replacement for the
    /// user's Pealayer palette. Derive its visible surfaces from that palette.
    pub fn with_visuals(mut self, visuals: &egui::Visuals) -> Self {
        self.app_bg = visuals.extreme_bg_color;
        self.header_bg = visuals.panel_fill;
        self.panel_bg = visuals.panel_fill;
        self.tab_bg = visuals.widgets.inactive.weak_bg_fill;
        self.text = visuals.text_color();
        self.text_dim = visuals.weak_text_color();
        self.text_faint = visuals.weak_text_color();
        self.tab_text = self.text_dim;
        self.tab_text_active = self.text;
        self.icon = self.text_dim;
        self.icon_active = self.text;
        self.hover = visuals.widgets.hovered.bg_fill;
        self.pressed = visuals.widgets.active.bg_fill;
        self.field_bg = visuals.extreme_bg_color;
        self.field_border = visuals.widgets.inactive.bg_stroke.color;
        self.separator = visuals.widgets.noninteractive.bg_stroke.color;
        self.well_bg = visuals.extreme_bg_color;
        self.card_bg = visuals.window_fill;
        self.border_subtle = visuals.window_stroke.color;
        self.row_alt = visuals.faint_bg_color;
        self.row_selected = visuals.selection.bg_fill;
        self.pill_active_bg = visuals.selection.bg_fill;
        self.clip_bar_bg = visuals.selection.bg_fill;
        self.control_surface = visuals.widgets.inactive.bg_fill;
        self.control_well = self.well_bg;
        self.control_border = self.field_border;
        self.button_hover = self.hover;
        self.button_border = self.field_border;
        self.tl_bg = visuals.panel_fill;
        self.tl_track_bg = visuals.panel_fill;
        self.tl_track_bg_alt = visuals.faint_bg_color;
        self.tl_header_bg = visuals.panel_fill;
        self.tl_ruler_bg = visuals.panel_fill;
        self.tl_ruler_tick = self.text_dim;
        self.tl_ruler_text = self.text_dim;
        self.timecode = self.text;
        self
    }

    /// Premiere-style dark/medium/light tokens measured from FilmCraft design system.
    pub fn for_kind(kind: ThemeKind) -> Self {
        let dark = Tokens {
            kind,
            app_bg: Color32::from_rgb(0, 0, 0),
            header_bg: Color32::from_rgb(0x1d, 0x1d, 0x1d),
            panel_bg: Color32::from_rgb(0x1d, 0x1d, 0x1d),
            tab_bg: Color32::from_rgb(0x1d, 0x1d, 0x1d),
            tab_text: Color32::from_rgb(0xb0, 0xb0, 0xb0),
            tab_text_active: Color32::from_rgb(0xd1, 0xd1, 0xd1),
            focus: Color32::from_rgb(0x57, 0x94, 0xec),
            accent: Color32::from_rgb(0x2f, 0x6b, 0xdf),
            accent_hover: Color32::from_rgb(0x3f, 0x7c, 0xe8),
            text: Color32::from_rgb(0xd1, 0xd1, 0xd1),
            text_dim: Color32::from_rgb(0xb0, 0xb0, 0xb0),
            text_faint: Color32::from_rgb(0x6e, 0x6e, 0x6e),
            icon: Color32::from_rgb(0xb0, 0xb0, 0xb0),
            icon_active: Color32::from_rgb(0xd1, 0xd1, 0xd1),
            hover: Color32::from_rgb(0x2c, 0x2c, 0x2c),
            pressed: Color32::from_rgb(0x4b, 0x4b, 0x4b),
            field_bg: Color32::from_rgb(0x0e, 0x0e, 0x0e),
            field_border: Color32::from_rgb(0x30, 0x30, 0x30),
            separator: Color32::from_rgb(0x24, 0x24, 0x24),
            well_bg: Color32::from_rgb(0x18, 0x18, 0x18),
            card_bg: Color32::from_rgb(0x24, 0x24, 0x24),
            border_subtle: Color32::from_rgb(0x30, 0x30, 0x30),
            modal_backdrop: Color32::from_rgba_premultiplied(0, 0, 0, 160),
            row_alt: Color32::from_rgb(0x21, 0x21, 0x21),
            row_selected: Color32::from_rgb(0x33, 0x33, 0x33),
            hot_text: Color32::from_rgb(0x40, 0x96, 0xf3),
            pill_active_bg: Color32::from_rgb(0x3a, 0x3a, 0x3a),
            clip_bar_bg: Color32::from_rgb(58, 58, 70),
            control_surface: Color32::from_rgb(0x24, 0x24, 0x24),
            control_well: Color32::from_rgb(0x10, 0x10, 0x10),
            control_border: Color32::from_rgb(0x50, 0x50, 0x50),
            control_handle_bg: Color32::from_rgb(0x2a, 0x2a, 0x2a),
            control_handle: Color32::from_rgb(0xe0, 0xe0, 0xe0),
            control_handle_dim: Color32::from_rgb(0xb0, 0xb0, 0xb0),
            plot_bg: Color32::from_rgb(0x18, 0x18, 0x18),
            plot_grid: Color32::from_rgb(0x2c, 0x2c, 0x2c),
            plot_axis: Color32::from_rgb(0x44, 0x44, 0x44),
            plot_handle: Color32::from_rgb(0xe0, 0xe0, 0xe0),
            plot_handle_dim: Color32::from_rgb(0x90, 0x90, 0x90),
            meter_bg: Color32::BLACK,
            meter_readout_bg: Color32::from_rgb(0x14, 0x14, 0x14),
            meter_clip_off: Color32::from_rgb(0x30, 0x30, 0x30),
            panner_bg: Color32::from_gray(22),
            panner_border: Color32::from_gray(96),
            panner_speaker: Color32::from_gray(144),
            fader_knob: Color32::from_gray(90),
            eq_track: Color32::from_gray(48),
            eq_tick: Color32::from_gray(96),
            eq_knob: Color32::from_gray(192),
            eq_knob_active: Color32::from_gray(240),
            eq_node: Color32::from_gray(232),
            eq_node_off: Color32::from_gray(112),
            crossover_handle: Color32::from_gray(255),
            keyframe_plot_bg: Color32::from_gray(25),
            keyframe_plot_grid: Color32::from_gray(42),
            keyframe_plot_axis: Color32::from_gray(51),
            keyframe_handle: Color32::from_gray(208),
            switch_fill: Color32::from_gray(212),
            switch_knob: Color32::from_gray(29),
            switch_border: Color32::from_gray(138),
            slider_disabled_track: Color32::from_gray(74),
            slider_disabled_knob: Color32::from_gray(106),
            button_hover: Color32::from_gray(42),
            button_border: Color32::from_gray(75),
            caption_track_bg: Color32::from_rgb(0x23, 0x20, 0x2a),
            scroll_thumb: Color32::from_gray(0x4b),
            scroll_thumb_hover: Color32::from_gray(0x6a),
            vertical_scroll_thumb: Color32::from_gray(80),
            disabled_clip_bg: Color32::from_gray(0x2a),
            tl_bg: Color32::from_rgb(0x1d, 0x1d, 0x1d),
            tl_track_bg: Color32::from_rgb(0x1d, 0x1d, 0x1d),
            tl_track_bg_alt: Color32::from_rgb(0x1d, 0x1d, 0x1d),
            tl_header_bg: Color32::from_rgb(0x1d, 0x1d, 0x1d),
            tl_ruler_bg: Color32::from_rgb(0x1d, 0x1d, 0x1d),
            tl_ruler_tick: Color32::from_rgb(0x8d, 0x8d, 0x8d),
            tl_ruler_text: Color32::from_rgb(0xb0, 0xb0, 0xb0),
            playhead: Color32::from_rgb(0x58, 0x95, 0xec),
            track_target: Color32::from_rgb(0x26, 0x5b, 0xc1),
            in_out_shade: Color32::from_rgb(0x3f, 0x3f, 0x3f),
            clip_selected_border: Color32::from_rgb(0xeb, 0xeb, 0xeb),
            render_red: Color32::from_rgb(0xe3, 0x48, 0x50),
            render_yellow: Color32::from_rgb(0xf0, 0xf0, 0x4f),
            render_green: Color32::from_rgb(0x2d, 0x9d, 0x78),
            monitor_bg: Color32::from_rgb(0x1d, 0x1d, 0x1d),
            timecode: Color32::from_rgb(0x40, 0x96, 0xf3),
            danger: Color32::from_rgb(0xdc, 0x51, 0x3d),
            radius: 0.0,
            radius_sm: 4.0,
            gap: 4.0,
            tab_h: 32.0,
        };

        match kind {
            ThemeKind::Dark => dark,
            ThemeKind::Medium => Tokens {
                kind: ThemeKind::Medium,
                app_bg: Color32::from_rgb(20, 20, 20),
                header_bg: Color32::from_rgb(0x32, 0x32, 0x32),
                panel_bg: Color32::from_rgb(0x32, 0x32, 0x32),
                tab_bg: Color32::from_rgb(0x32, 0x32, 0x32),
                field_bg: Color32::from_rgb(0x22, 0x22, 0x22),
                tl_bg: Color32::from_rgb(0x32, 0x32, 0x32),
                tl_track_bg: Color32::from_rgb(0x32, 0x32, 0x32),
                tl_track_bg_alt: Color32::from_rgb(0x32, 0x32, 0x32),
                tl_header_bg: Color32::from_rgb(0x32, 0x32, 0x32),
                tl_ruler_bg: Color32::from_rgb(0x32, 0x32, 0x32),
                monitor_bg: Color32::from_rgb(0x32, 0x32, 0x32),
                row_alt: Color32::from_rgb(50, 50, 50),
                hover: Color32::from_rgb(64, 64, 64),
                separator: Color32::from_rgb(0x3a, 0x3a, 0x3a),
                well_bg: Color32::from_rgb(0x28, 0x28, 0x28),
                card_bg: Color32::from_rgb(0x38, 0x38, 0x38),
                border_subtle: Color32::from_rgb(0x40, 0x40, 0x40),
                ..dark
            },
            ThemeKind::Light => Tokens {
                kind: ThemeKind::Light,
                app_bg: Color32::from_rgb(180, 180, 180),
                header_bg: Color32::from_rgb(214, 214, 214),
                panel_bg: Color32::from_rgb(232, 232, 232),
                tab_bg: Color32::from_rgb(232, 232, 232),
                tab_text: Color32::from_rgb(90, 90, 90),
                tab_text_active: Color32::from_rgb(20, 20, 20),
                text: Color32::from_rgb(34, 34, 34),
                text_dim: Color32::from_rgb(90, 90, 90),
                text_faint: Color32::from_rgb(140, 140, 140),
                icon: Color32::from_rgb(60, 60, 60),
                hover: Color32::from_rgb(210, 210, 210),
                pressed: Color32::from_rgb(196, 196, 196),
                field_bg: Color32::from_rgb(250, 250, 250),
                field_border: Color32::from_rgb(170, 170, 170),
                separator: Color32::from_rgb(200, 200, 200),
                well_bg: Color32::from_rgb(240, 240, 240),
                card_bg: Color32::from_rgb(255, 255, 255),
                border_subtle: Color32::from_rgb(210, 210, 210),
                modal_backdrop: Color32::from_rgba_premultiplied(0, 0, 0, 100),
                row_alt: Color32::from_rgb(224, 224, 224),
                row_selected: Color32::from_rgb(170, 200, 240),
                pill_active_bg: Color32::from_rgb(196, 196, 196),
                clip_bar_bg: Color32::from_rgb(190, 203, 224),
                control_surface: Color32::from_rgb(218, 218, 218),
                control_well: Color32::from_rgb(196, 196, 196),
                control_border: Color32::from_rgb(150, 150, 150),
                control_handle_bg: Color32::from_rgb(205, 205, 205),
                control_handle: Color32::from_rgb(50, 50, 50),
                control_handle_dim: Color32::from_rgb(85, 85, 85),
                plot_bg: Color32::from_rgb(245, 245, 245),
                plot_grid: Color32::from_rgb(210, 210, 210),
                plot_axis: Color32::from_rgb(170, 170, 170),
                plot_handle: Color32::from_rgb(50, 50, 50),
                plot_handle_dim: Color32::from_rgb(100, 100, 100),
                meter_bg: Color32::from_rgb(205, 205, 205),
                meter_readout_bg: Color32::from_rgb(218, 218, 218),
                meter_clip_off: Color32::from_rgb(155, 155, 155),
                panner_bg: Color32::from_gray(196),
                panner_border: Color32::from_gray(150),
                panner_speaker: Color32::from_gray(100),
                fader_knob: Color32::from_gray(140),
                eq_track: Color32::from_gray(196),
                eq_tick: Color32::from_gray(150),
                eq_knob: Color32::from_gray(85),
                eq_knob_active: Color32::from_gray(34),
                eq_node: Color32::from_gray(50),
                eq_node_off: Color32::from_gray(140),
                crossover_handle: Color32::from_gray(50),
                keyframe_plot_bg: Color32::from_gray(245),
                keyframe_plot_grid: Color32::from_gray(210),
                keyframe_plot_axis: Color32::from_gray(170),
                keyframe_handle: Color32::from_gray(50),
                switch_fill: Color32::from_gray(60),
                switch_knob: Color32::from_gray(232),
                switch_border: Color32::from_gray(110),
                slider_disabled_track: Color32::from_gray(190),
                slider_disabled_knob: Color32::from_gray(140),
                button_hover: Color32::from_gray(210),
                button_border: Color32::from_gray(170),
                icon_active: Color32::from_rgb(34, 34, 34),
                caption_track_bg: Color32::from_rgb(222, 215, 233),
                scroll_thumb: Color32::from_gray(150),
                scroll_thumb_hover: Color32::from_gray(130),
                vertical_scroll_thumb: Color32::from_gray(150),
                disabled_clip_bg: Color32::from_gray(190),
                in_out_shade: Color32::from_gray(185),
                tl_bg: Color32::from_rgb(210, 210, 210),
                tl_track_bg: Color32::from_rgb(222, 222, 222),
                tl_track_bg_alt: Color32::from_rgb(228, 228, 228),
                tl_header_bg: Color32::from_rgb(214, 214, 214),
                tl_ruler_bg: Color32::from_rgb(226, 226, 226),
                tl_ruler_text: Color32::from_rgb(80, 80, 80),
                ..dark
            },
        }
    }

    /// Adapts tokens with a custom accent color or accessible contrast.
    pub fn with_appearance(mut self, highlight: Option<[u8; 3]>, accessible_contrast: bool) -> Self {
        if let Some([r, g, b]) = highlight {
            let c = Color32::from_rgb(r, g, b);
            if c != self.accent {
                let tint = |s: f32, v: f32| {
                    let mut h = egui::ecolor::HsvaGamma::from(c);
                    h.s = (h.s * s).clamp(0.0, 1.0);
                    h.v = (h.v * v).clamp(0.0, 1.0);
                    Color32::from(h)
                };
                self.focus = tint(0.92, 1.06);
                self.playhead = self.focus;
                self.timecode = tint(0.98, 1.09);
                self.hot_text = self.timecode;
                self.track_target = tint(1.0, 0.865);
            }
            let lift = |v: u8| v.saturating_add(16);
            self.accent = c;
            self.accent_hover = Color32::from_rgb(lift(r), lift(g), lift(b));
        }
        if accessible_contrast {
            if self.kind == ThemeKind::Light {
                self.text_dim = Color32::from_rgb(40, 40, 40);
                self.text_faint = Color32::from_rgb(80, 80, 80);
                self.field_border = Color32::from_rgb(110, 110, 110);
            } else {
                self.text_dim = self.text;
                self.text_faint = Color32::from_rgb(0xa8, 0xa8, 0xa8);
                self.tab_text = self.tab_text_active;
                self.field_border = Color32::from_rgb(0x6a, 0x6a, 0x6a);
                self.separator = Color32::from_rgb(0x50, 0x50, 0x50);
                self.border_subtle = Color32::from_rgb(0x60, 0x60, 0x60);
            }
        }
        self
    }

    /// Constructs a `craft_ui::theme::WidgetPalette` from these design tokens.
    pub fn widget_palette(&self) -> craft_ui::theme::WidgetPalette {
        craft_ui::theme::WidgetPalette {
            noninteractive: self.panel_bg,
            inactive: self.field_bg,
            hovered: self.hover,
            active: self.pressed,
            open: self.hover,
        }
    }

    /// Constructs an `IconButtonStyle` matching FilmCraft button tokens.
    pub fn icon_button_style(&self) -> craft_ui::buttons::IconButtonStyle {
        icon_button_style(self, false)
    }

    /// Constructs a `ChromeStyle` matching FilmCraft floating and task pane chrome.
    pub fn chrome_style(&self) -> craft_ui::docking::chrome::ChromeStyle {
        craft_ui::docking::chrome::ChromeStyle {
            background: self.panel_bg,
            stroke: Stroke::new(1.0, self.separator),
            title_background: self.tab_bg,
            text: self.text,
            weak_text: self.text_dim,
            accent: self.accent,
            active_background: self.pressed,
            font: Self::ui_font(12.0),
            right_to_left: false,
        }
    }

    /// Constructs a `ResizeHandleStyle` matching FilmCraft split dividers.
    pub fn resize_handle_style(&self) -> craft_ui::layout::ResizeHandleStyle {
        craft_ui::layout::ResizeHandleStyle {
            hovered: self.focus,
            dragged: self.accent_hover,
            focused: self.focus,
        }
    }

    /// Constructs a `DockStyle` matching FilmCraft workspace docking.
    pub fn craft_dock_style(&self) -> craft_ui::docking::DockStyle {
        let visuals = if self.kind == ThemeKind::Light {
            Visuals::light()
        } else {
            Visuals::dark()
        };
        let mut ds = craft_ui::docking::DockStyle::from_visuals(&visuals);
        ds.tab_height = self.tab_h;
        ds.gap = self.gap;
        ds.background = self.panel_bg;
        ds.tab_background = self.tab_bg;
        ds.active_background = self.pressed;
        ds.text = self.text;
        ds.inactive_text = self.text_dim;
        ds.border = Stroke::new(1.0, self.separator);
        ds.accent = self.accent;
        ds.floating_background = self.panel_bg;
        ds.floating_stroke = Stroke::new(1.0, self.separator);
        ds
    }

    /// Font helper for monospace labels / timecode.
    pub fn mono(size: f32) -> FontId {
        FontId::new(size, FontFamily::Monospace)
    }

    /// Font helper for standard proportional UI text.
    pub fn ui_font(size: f32) -> FontId {
        FontId::new(size, FontFamily::Proportional)
    }

    /// Font helper for semibold section headings.
    pub fn semibold(size: f32) -> FontId {
        FontId::new(size, FontFamily::Name("semibold".into()))
    }
}

pub const MENU_TEXT_SIZE: f32 = 13.0;
pub const MENU_TEXT: Color32 = Color32::from_rgb(0xde, 0xde, 0xde);
pub const ANIMATION_TIME: f32 = 0.12;

// Re-export Craft UI layout and chrome dimension tokens
pub const DOCK_SNAP: f32 = 32.0;
pub const TITLE_BAR_HEIGHT: f32 = 24.0;
pub const RAIL_WIDTH: f32 = 48.0;
pub const RAIL_BUTTON: f32 = 34.0;
pub const TAB_PADDING: f32 = 24.0;
pub const TITLE_HEIGHT: f32 = 14.0;
pub const RESIZE_BAND: f32 = 4.0;

/// Constructs FilmCraft visuals from design tokens.
pub fn build_visuals(t: &Tokens) -> Visuals {
    let light = t.kind == ThemeKind::Light;
    let mut v = if light { Visuals::light() } else { Visuals::dark() };
    v.panel_fill = t.panel_bg;
    v.window_fill = t.panel_bg;
    v.extreme_bg_color = t.field_bg;
    v.faint_bg_color = t.row_alt;
    v.override_text_color = Some(t.text);
    v.selection.bg_fill = t.accent;
    v.selection.stroke = Stroke::new(1.0, Color32::WHITE);
    v.hyperlink_color = t.accent;
    v.window_stroke = Stroke::new(1.0, t.field_border);
    v.window_corner_radius = CornerRadius::same(6);
    v.menu_corner_radius = CornerRadius::same(6);
    v.popup_shadow = egui::epaint::Shadow {
        offset: [0, 4],
        blur: 16,
        spread: 0,
        color: Color32::from_black_alpha(140),
    };
    v.window_shadow = v.popup_shadow;

    // Apply craft-ui widget palette across all 5 widget interaction states
    let widget_palette = t.widget_palette();
    widget_palette.apply_to(
        &mut v.widgets,
        CornerRadius::same(t.radius_sm as u8),
        Stroke::new(1.0, t.text),
    );

    // Apply FilmCraft specific borders and hover strokes
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, t.separator);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, t.field_border);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, t.field_border);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, t.tab_text_active);
    v.widgets.active.fg_stroke = Stroke::new(1.0, t.tab_text_active);
    v
}

/// Applies FilmCraft visuals and spacing tokens to the egui context.
pub fn apply_visuals(ctx: &egui::Context, t: &Tokens, density: crate::config::UiControlDensity) {
    let light = t.kind == ThemeKind::Light;
    ctx.set_theme(if light { egui::Theme::Light } else { egui::Theme::Dark });

    let v = build_visuals(t);
    ctx.set_visuals(v);

    // Store active tokens in context data for fast component access
    set_current(ctx, *t);

    ctx.global_style_mut(|s| {
        // FilmCraft spacing tokens
        s.spacing.item_spacing = Vec2::new(6.0, 4.0);
        s.spacing.button_padding = Vec2::new(8.0, 3.0);
        s.spacing.menu_margin = Margin::same(4);
        s.spacing.interact_size.y = density.base_height();
        s.animation_time = ANIMATION_TIME;

        // FilmCraft typography tokens
        s.text_styles.insert(TextStyle::Body, FontId::new(12.0, FontFamily::Proportional));
        s.text_styles.insert(TextStyle::Button, FontId::new(12.0, FontFamily::Proportional));
        s.text_styles.insert(TextStyle::Small, FontId::new(11.0, FontFamily::Proportional));
        s.text_styles.insert(TextStyle::Heading, FontId::new(15.0, FontFamily::Name("semibold".into())));
        s.text_styles.insert(TextStyle::Monospace, FontId::new(12.0, FontFamily::Monospace));
    });
}

fn tokens_context_id() -> egui::Id {
    egui::Id::new("pealayer-active-craft-tokens")
}

/// Retrieves the active `Tokens` stored in the egui context, or returns the default dark tokens.
pub fn current(ctx: &egui::Context) -> Tokens {
    // Read the theme outside the context-data transaction (non-reentrant lock).
    let light = ctx.theme() == egui::Theme::Light;
    let (themes, explicit) = ctx.data(|data| (
        data.get_temp::<(Tokens, Tokens)>(tokens_context_id().with("themes")),
        data.get_temp::<Tokens>(tokens_context_id()),
    ));
    themes.map(|(dark, day)| if light { day } else { dark })
        .or(explicit)
        .unwrap_or_else(|| Tokens::for_kind(if light { ThemeKind::Light } else { ThemeKind::Dark }))
}

/// Stores the active `Tokens` in the egui context.
pub fn set_current(ctx: &egui::Context, tokens: Tokens) {
    ctx.data_mut(|data| {
        data.remove::<(Tokens, Tokens)>(tokens_context_id().with("themes"));
        data.insert_temp(tokens_context_id(), tokens);
    });
}

/// Both palettes remain available when System delivers a live theme event.
pub fn set_themes(ctx: &egui::Context, dark: Tokens, light: Tokens) {
    ctx.data_mut(|data| data.insert_temp(tokens_context_id().with("themes"), (dark, light)));
}

/// Applies FilmCraft styling to a `craft_ui::docking::DockStyle`.
pub fn apply_dock_style(style: &mut craft_ui::docking::DockStyle, t: &Tokens) {
    style.tab_height = t.tab_h;
    style.background = t.panel_bg;
    style.tab_background = t.tab_bg;
    style.active_background = t.panel_bg;
    style.text = t.tab_text_active;
    style.inactive_text = t.tab_text;
    style.border = Stroke::new(1.0, t.separator);
    style.accent = t.focus;
    style.floating_background = t.panel_bg;
    style.floating_stroke = Stroke::new(1.0, t.separator);
}

/// Constructs an `IconButtonStyle` matching FilmCraft button tokens.
pub fn icon_button_style(tokens: &Tokens, frame: bool) -> craft_ui::buttons::IconButtonStyle {
    craft_ui::buttons::IconButtonStyle {
        normal: craft_ui::buttons::ButtonVisuals {
            fill: if frame { tokens.panel_bg } else { Color32::TRANSPARENT },
            stroke: if frame { Stroke::new(1.0, tokens.field_border) } else { Stroke::NONE },
            icon: tokens.text_dim,
        },
        hovered: craft_ui::buttons::ButtonVisuals {
            fill: tokens.hover,
            stroke: if frame { Stroke::new(1.0, tokens.field_border) } else { Stroke::NONE },
            icon: tokens.text,
        },
        pressed: Some(craft_ui::buttons::ButtonVisuals {
            fill: tokens.pressed,
            stroke: Stroke::new(1.0, tokens.accent),
            icon: tokens.text,
        }),
        selected: craft_ui::buttons::ButtonVisuals {
            fill: tokens.pill_active_bg,
            stroke: Stroke::new(1.0, tokens.accent),
            icon: super::accent_foreground(tokens.pill_active_bg),
        },
        corner_radius: CornerRadius::same(4),
        focus_stroke: Stroke::new(1.0, tokens.focus),
    }
}

/// Paints the FilmCraft active panel outline around the given rect.
pub fn paint_active_panel_outline(painter: &egui::Painter, rect: egui::Rect, t: &Tokens) {
    painter.rect_stroke(rect, t.radius, Stroke::new(1.0, t.focus), StrokeKind::Inside);
}

/// Paints the FilmCraft widget focus outline (2px inside stroke) when focused.
pub fn paint_widget_focus(painter: &egui::Painter, response: &egui::Response, t: &Tokens) {
    if response.has_focus() {
        painter.rect_stroke(response.rect.shrink(2.0), 2.0, Stroke::new(1.5, t.focus), StrokeKind::Inside);
    }
}

/// Shows an `IconButton` using FilmCraft tokens and standard widget radius (4px).
pub fn icon_button(
    ui: &mut egui::Ui,
    label: &str,
    size: egui::Vec2,
    selected: bool,
    frame: bool,
    paint_icon: impl FnOnce(&egui::Ui, egui::Rect, egui::Color32),
) -> egui::Response {
    let tokens = current(ui.ctx());
    let style = icon_button_style(&tokens, frame);
    let mut btn = craft_ui::buttons::IconButton::new(label, size, &style).selected(selected);
    if frame {
        btn = btn.single_shape_frame();
    }
    btn.show(ui, paint_icon)
}

/// Shows an `IconButton` with a text or Phosphor icon glyph.
pub fn icon_button_glyph(
    ui: &mut egui::Ui,
    label: &str,
    glyph: &str,
    size: egui::Vec2,
    selected: bool,
    frame: bool,
) -> egui::Response {
    icon_button(ui, label, size, selected, frame, |ui, rect, color| {
        let font = egui::TextStyle::Button.resolve(ui.style());
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            glyph,
            font,
            color,
        );
    })
}

/// The Ruler's two strides, in frames: `(minor_stride, label_stride)`.
///
/// `label_step` is the first ladder step wide enough to keep timestamps apart (110 pt); `minor` is the
/// finest divisor of `label_step` that still leaves ticks 12 pt apart. Deriving the tick stride as a
/// divisor puts every label on a tick and keeps the ticks between labels evenly spaced.
pub fn ruler_steps(frame_px: f64, base: i64) -> (i64, i64) {
    let base = base.max(1);
    let big = |k: i64| base.saturating_mul(k);
    let steps = [
        1, 2, 5, 10,
        (base / 2).max(1),
        base,
        big(2), big(5), big(10), big(15), big(30), big(60),
        big(120), big(300), big(600), big(1800), big(3600),
    ];
    let label_step = (*steps.iter().find(|s| **s as f64 * frame_px >= 110.0).unwrap_or(&big(3600))).max(1);
    let minor = (1..=9)
        .rev()
        .filter(|k| label_step.rem_euclid(*k) == 0)
        .map(|k| label_step / k)
        .find(|m| *m as f64 * frame_px >= 12.0)
        .unwrap_or(label_step)
        .max(1);
    (minor, label_step)
}

/// FilmCraft's frame ladder positions frames at the actual media rate. The
/// nominal rate only chooses readable strides; it does not change media timing.
pub fn paint_frame_ruler(painter: &egui::Painter, rect: egui::Rect, origin_x: f32,
    points_per_second: f32, media_fps: f64, tokens: &Tokens) {
    if !points_per_second.is_finite() || points_per_second <= 0.0 { return; }
    let fps = if media_fps.is_finite() && media_fps > 0.0 { media_fps.min(1000.0) } else { 30.0 };
    let frame_px = f64::from(points_per_second) / fps;
    let (minor, major) = ruler_steps(frame_px, fps.round() as i64);
    let clip = rect.intersect(painter.clip_rect());
    let first = (((clip.left() - origin_x).max(0.0) as f64 / frame_px) as i64 / minor) * minor;
    let last = (((clip.right() - origin_x).max(0.0) as f64 / frame_px).ceil() as i64).saturating_add(minor);
    let font = egui::FontId::monospace(9.0);
    for frame in (first..=last).step_by(minor as usize).take(4096) {
        let x = origin_x + (frame as f64 * frame_px) as f32;
        let labeled = frame % major == 0;
        let height = if labeled { 8.0 } else { 3.5 };
        painter.line_segment([egui::pos2(x, rect.bottom()-height), egui::pos2(x, rect.bottom())],
            Stroke::new(1.0, tokens.tl_ruler_tick.gamma_multiply(if labeled { 1.0 } else { 0.6 })));
        if labeled {
            let ms = (frame as f64 * 1000.0 / fps).round().max(0.0) as u64;
            let text = crate::duration::format_timeline_time_ms(ms,
                major as f64 / fps < 1.0 || fps.fract().abs() > 0.0001);
            let galley = painter.layout_no_wrap(text, font.clone(), tokens.tl_ruler_text);
            let bounds = egui::Rect::from_center_size(egui::pos2(x, rect.top()+13.0), galley.size());
            if clip.contains_rect(bounds) { painter.galley(bounds.min, galley, tokens.tl_ruler_text); }
        }
    }
}

/// How far the Timeline reaches, in seconds: ten minutes past the end of the sequence (FilmCraft standard).
/// FilmCraft's original monitor track and terminal rings, painted over the
/// real accessible slider. All overlays use its actual value range.
pub fn paint_monitor_seek(ui: &egui::Ui, response: &egui::Response, fraction: f32, buffered: Option<f32>) {
    let tokens = current(ui.ctx());
    let color = |c: Color32| c.gamma_multiply(if response.enabled() {1.0} else {0.4});
    let range = super::controls::seekbar_value_range(response.rect);
    if range.end() <= range.start() { return; }
    let track = egui::Rect::from_min_max(egui::pos2(*range.start(),response.rect.center().y-4.0),
        egui::pos2(*range.end(),response.rect.center().y+4.0));
    let p = ui.painter().with_clip_rect(response.rect.intersect(ui.clip_rect()));
    p.rect_filled(track,4.0,color(tokens.separator));
    let x = egui::lerp(range.clone(),fraction.clamp(0.0,1.0));
    if let Some(end) = buffered.filter(|end|end.is_finite() && *end > fraction) {
        p.rect_filled(egui::Rect::from_min_max(egui::pos2(x,track.top()),
            egui::pos2(egui::lerp(range.clone(),end.clamp(0.0,1.0)),track.bottom())),2.0,color(tokens.accent.gamma_multiply(0.3)));
    }
    for edge in [*range.start(),*range.end()] {
        p.circle_filled(egui::pos2(edge,track.center().y),3.5,color(tokens.panel_bg));
        p.circle_stroke(egui::pos2(edge,track.center().y),3.5,Stroke::new(1.25,color(tokens.icon_active)));
    }
    let top = response.rect.top()+1.0;
    paint_filmcraft_playhead(&p,&p,x,top,top+8.0,track.bottom(),color(tokens.playhead));
    if response.has_focus() { p.rect_stroke(response.rect.shrink(0.5),3.0,Stroke::new(1.0,tokens.focus),StrokeKind::Inside); }
}

pub fn paint_monitor_volume(ui: &egui::Ui, response: &egui::Response, fraction: f32) {
    let t = current(ui.ctx());
    let color = |c: Color32| c.gamma_multiply(if response.enabled() {1.0} else {0.4});
    let range = super::controls::seekbar_value_range(response.rect);
    if range.end() <= range.start() { return; }
    let x = egui::lerp(range.clone(),fraction.clamp(0.0,1.0));
    let p=ui.painter().with_clip_rect(response.rect.intersect(ui.clip_rect()));
    let track=egui::Rect::from_min_max(egui::pos2(*range.start(),response.rect.center().y-2.0),egui::pos2(*range.end(),response.rect.center().y+2.0));
    p.rect_filled(track,2.0,color(t.separator));
    p.rect_filled(egui::Rect::from_min_max(track.min,egui::pos2(x,track.bottom())),2.0,color(t.accent));
    p.circle_filled(egui::pos2(x,track.center().y),4.5,color(t.panel_bg));
    p.circle_stroke(egui::pos2(x,track.center().y),4.5,Stroke::new(1.5,color(if response.hovered() || response.dragged() {t.tab_text_active} else {t.icon_active})));
    if response.has_focus() { p.rect_stroke(response.rect.shrink(0.5),3.0,Stroke::new(1.0,t.focus),StrokeKind::Inside); }
}

/// The original two-point overview represents buffered media, not rendered
/// frames. No buffer observation means no fabricated green progress.
pub fn paint_buffered_overview(p: &egui::Painter, ruler: egui::Rect, origin_x:f32, pps:f32,
    buffered_until:Option<f64>, t:&Tokens) {
    if let Some(end)=buffered_until.filter(|v|v.is_finite() && *v>0.0)
        && pps.is_finite() && pps>0.0 {
        let end=origin_x+end as f32*pps;
        let bar=egui::Rect::from_min_max(egui::pos2(origin_x,ruler.bottom()-2.0),egui::pos2(end,ruler.bottom())).intersect(ruler);
        if bar.is_positive() { p.with_clip_rect(ruler.intersect(p.clip_rect())).rect_filled(bar,0.0,t.render_green); }
    }
}

pub fn timeline_extent(sequence_seconds: f64) -> f64 {
    sequence_seconds.max(0.0) + 600.0
}

/// The least `pps` (points a second) the Timeline zooms out to.
pub fn min_pps(sequence_seconds: f64, width: f32) -> f64 {
    (f64::from(width.max(0.0)) / timeline_extent(sequence_seconds)).clamp(1e-6, 24_000.0)
}

/// The narrowest the zoom scroll bar's thumb gets: room for its two handles and a part between them to grab.
pub const MIN_THUMB_W: f32 = 34.0;

/// What a pointer is over on a zoom scroll bar (the Timeline's and the monitors'): the circular
/// handle at either end of the thumb zooms, the middle scrolls, and a press beside the thumb brings
/// it there. The press and the hover cursor ask the same question here, so they cannot disagree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoomThumbPart {
    Beside,
    Left,
    Middle,
    Right,
}

impl ZoomThumbPart {
    pub fn at(thumb: egui::Rect, pos: egui::Pos2) -> Self {
        if !thumb.expand2(egui::vec2(0.0, 4.0)).contains(pos) {
            Self::Beside
        } else if (pos.x - thumb.min.x).abs() < 9.0 {
            Self::Left
        } else if (pos.x - thumb.max.x).abs() < 9.0 {
            Self::Right
        } else {
            Self::Middle
        }
    }
}

/// The horizontal resize cursor of a zoom scroll bar: over either handle of its `thumb`, and for as
/// long as a drag of a handle goes on (`handle_grabbed`), which the pointer can take off the bar.
pub fn zoom_thumb_cursor(ui: &egui::Ui, resp: &egui::Response, thumb: egui::Rect, handle_grabbed: bool) {
    let on_handle = if resp.dragged() {
        handle_grabbed
    } else {
        resp.hover_pos().is_some_and(|pos| matches!(ZoomThumbPart::at(thumb, pos), ZoomThumbPart::Left | ZoomThumbPart::Right))
    };
    if on_handle {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }
}

/// Where the zoom scroll bar's thumb is: (left edge, width, seconds of scroll per point of thumb travel).
pub fn zoom_thumb(bar: egui::Rect, seq_seconds: f64, scroll: f64, pps: f64) -> (f32, f32, f64) {
    let vis = bar.width() as f64 / pps.max(1e-9);
    let total = timeline_extent(seq_seconds).max(vis);
    let w = ((vis / total) as f32 * bar.width()).clamp(MIN_THUMB_W.min(bar.width()), bar.width());
    let travel = (bar.width() - w).max(0.0);
    let range = (total - vis).max(0.0);
    let at = if range > 0.0 { (scroll / range).clamp(0.0, 1.0) as f32 * travel } else { 0.0 };
    (bar.min.x + at, w, if travel > 0.5 { range / travel as f64 } else { 0.0 })
}

/// Paint FilmCraft's zoom scroll bar thumb with dual-circle handles.
pub fn paint_zoom_thumb(
    p: &egui::Painter,
    bar: egui::Rect,
    thumb: egui::Rect,
    hover: bool,
    tokens: &Tokens,
) {
    // track
    p.rect_filled(bar, bar.height() / 2.0, tokens.separator);
    // thumb
    p.rect_filled(
        thumb,
        thumb.height() / 2.0,
        if hover { tokens.scroll_thumb_hover } else { tokens.scroll_thumb },
    );
    // dual circular handles
    for x in [
        thumb.min.x + thumb.height() / 2.0,
        thumb.max.x - thumb.height() / 2.0,
    ] {
        p.circle_filled(egui::pos2(x, thumb.center().y), 4.5, tokens.panel_bg);
        p.circle_stroke(
            egui::pos2(x, thumb.center().y),
            4.5,
            Stroke::new(1.5, tokens.icon_active),
        );
    }
}

/// Paint FilmCraft's precision pentagon playhead and continuous subpixel stem line.
pub fn paint_filmcraft_playhead(
    painter: &egui::Painter,
    ruler_painter: &egui::Painter,
    px: f32,
    top: f32,
    tip: f32,
    track_bottom: f32,
    color: Color32,
) {
    // 10 pt wide, 13 pt tall pentagonal head with 5 pt downward tip
    let head = [
        egui::pos2(px - 5.0, top),
        egui::pos2(px + 5.0, top),
        egui::pos2(px + 5.0, tip - 5.0),
        egui::pos2(px, tip),
        egui::pos2(px - 5.0, tip - 5.0),
    ];
    ruler_painter.add(egui::Shape::convex_polygon(head.to_vec(), color, Stroke::NONE));
    // A path, not a line segment: egui rounds line segments to whole pixels, so while playing
    // the line would move in uneven pixel steps and part from its head
    painter.add(egui::Shape::line(
        vec![egui::pos2(px, tip), egui::pos2(px, track_bottom)],
        Stroke::new(1.0, color),
    ));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkerPart {
    Whole,
    Left,
    Right,
}

/// Paint FilmCraft's precision pentagonal marker glyph (chapters, sequence markers, keyframes).
pub fn marker_glyph(
    painter: &egui::Painter,
    rect: egui::Rect,
    tip_height: f32,
    part: MarkerPart,
    color: Color32,
) {
    let x = rect.center().x;
    let y = rect.top();
    let shoulder = rect.bottom() - tip_height;
    let points = match part {
        MarkerPart::Whole => vec![
            egui::pos2(rect.left(), y),
            egui::pos2(rect.right(), y),
            egui::pos2(rect.right(), shoulder),
            egui::pos2(x, rect.bottom()),
            egui::pos2(rect.left(), shoulder),
        ],
        MarkerPart::Left => vec![
            egui::pos2(rect.left(), y),
            egui::pos2(x, y),
            egui::pos2(x, rect.bottom()),
            egui::pos2(rect.left(), shoulder),
        ],
        MarkerPart::Right => vec![
            egui::pos2(x, y),
            egui::pos2(rect.right(), y),
            egui::pos2(rect.right(), shoulder),
            egui::pos2(x, rect.bottom()),
        ],
    };
    painter.add(egui::Shape::convex_polygon(points, color, Stroke::NONE));
}

/// FilmCraft dropdown text selector ("Fit ▾", "1/2 ▾") with 4px corner radius and field tokens.
pub fn dropdown_text(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    text: &str,
    t: &Tokens,
    id: egui::Id,
) -> egui::Response {
    let resp = ui.interact(rect, id, egui::Sense::click());
    ui.painter().rect_filled(rect, 4.0, t.field_bg);
    ui.painter().rect_stroke(
        rect,
        4.0,
        Stroke::new(
            1.0,
            if resp.hovered() { t.accent } else { t.field_border },
        ),
        egui::StrokeKind::Inside,
    );
    let rtl = ui.layout().main_dir() == egui::Direction::RightToLeft;
    let font = egui::TextStyle::Body.resolve(ui.style());
    let label_rect = if rtl { rect.shrink2(egui::vec2(8.0, 0.0)).translate(egui::vec2(12.0, 0.0)) }
        else { rect.shrink2(egui::vec2(8.0, 0.0)) };
    let painter = ui.painter().with_clip_rect(label_rect.intersect(rect).intersect(ui.clip_rect()));
    painter.text(egui::pos2(if rtl { rect.right()-9.0 } else { rect.left()+9.0 }, rect.center().y),
        if rtl { egui::Align2::RIGHT_CENTER } else { egui::Align2::LEFT_CENTER }, text, font, t.text);
    let chevron_x = if rtl { rect.left()+11.0 } else { rect.right()-11.0 };
    super::control_density::paint_chevron(ui,
        egui::Rect::from_center_size(egui::pos2(chevron_x, rect.center().y), egui::vec2(14.0,14.0)), t.text_dim);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, resp.enabled(), text));
    resp
}

#[cfg(test)]
mod tests {
    #[test]
    fn filmcraft_restored_monitor_art_has_real_pentagon_rings_and_bounded_paint() {
        for scale in [1.0,1.5,2.0] {
            let ctx=egui::Context::default();ctx.set_pixels_per_point(scale);
            let output=ctx.run_ui(egui::RawInput::default(),|ui| {
                let (_,response)=ui.allocate_exact_size(egui::vec2(400.0,28.0),egui::Sense::click_and_drag());
                super::paint_monitor_seek(ui,&response,0.4375,Some(0.75));
                super::paint_monitor_volume(ui,&response,0.5);
            });
            assert!(output.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Path(p) if p.closed && p.points.len()==5)),"live monitor lost its pentagonal playhead");
            assert!(output.shapes.iter().filter(|s|matches!(&s.shape,egui::Shape::Circle(_))).count()>=6,"terminal and volume rings lost");
            output.drop_without_applying_deltas();
        }
    }
    use super::*;

    #[test]
    fn filmcraft_ruler_labels_are_centered_and_fractional_frames_use_actual_rate() {
        let ctx = egui::Context::default();
        let tokens = Tokens::for_kind(ThemeKind::Dark);
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 30.0));
        let output = ctx.run_ui(egui::RawInput::default(), |ui| {
            paint_frame_ruler(&ui.painter().with_clip_rect(rect), rect, 0.0, 100.0, 23.976, &tokens);
        });
        let ticks: Vec<_> = output.shapes.iter().filter_map(|shape| match &shape.shape {
            egui::Shape::LineSegment { points, .. } => Some(points[0].x), _ => None,
        }).collect();
        assert!(ticks.len() > 10);
        // Nominal 24-frame major tick is slightly after 1s at 23.976fps.
        let expected = (24.0 / 23.976 * 100.0) as f32;
        assert!(ticks.iter().any(|x| (*x-expected).abs() < 0.01));
        let mut labels = 0;
        for shape in &output.shapes {
            if let egui::Shape::Text(text) = &shape.shape {
                let center = text.pos.x + text.galley.size().x/2.0;
                assert!(ticks.iter().any(|x| (*x-center).abs() < 0.01));
                assert!(rect.contains_rect(egui::Rect::from_min_size(text.pos, text.galley.size())));
                labels += 1;
            }
        }
        assert!(labels >= 3);
        output.drop_without_applying_deltas();
    }

    #[test]
    fn filmcraft_playhead_head_stays_in_ruler_and_stem_begins_at_track_boundary() {
        let ctx = egui::Context::default();
        let output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let ruler = egui::Rect::from_min_max(egui::pos2(0.0,0.0), egui::pos2(800.0,30.0));
            paint_filmcraft_playhead(ui.painter(), &ui.painter().with_clip_rect(ruler),
                150.25, 17.0, 30.0, 300.0, Color32::RED);
        });
        let paths: Vec<_> = output.shapes.iter().filter_map(|shape| match &shape.shape {
            egui::Shape::Path(path) => Some(path), _ => None,
        }).collect();
        assert_eq!(paths.len(), 2);
        assert!(paths[0].points.iter().all(|point| point.y >= 17.0 && point.y <= 30.0));
        assert_eq!(paths[1].points, vec![egui::pos2(150.25,30.0), egui::pos2(150.25,300.0)]);
        output.drop_without_applying_deltas();
    }

    #[test]
    fn filmcraft_subsecond_ruler_never_repeats_whole_second_labels() {
        let ctx = egui::Context::default();
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0,30.0));
        let output = ctx.run_ui(egui::RawInput::default(), |ui| {
            paint_frame_ruler(&ui.painter().with_clip_rect(rect), rect, 0.0, 500.0, 24.0,
                &Tokens::for_kind(ThemeKind::Dark));
        });
        let labels: Vec<_> = output.shapes.iter().filter_map(|shape| match &shape.shape {
            egui::Shape::Text(text) => Some(text.galley.text().to_owned()), _ => None,
        }).collect();
        assert!(labels.len() >= 3);
        assert!(labels.windows(2).all(|pair| pair[0] != pair[1]), "{labels:?}");
        assert!(labels.iter().any(|label| label.contains('.')));
        output.drop_without_applying_deltas();
    }

    #[test]
    fn selected_craft_icons_contrast_with_bright_and_dark_fills() {
        let mut tokens = Tokens::for_kind(ThemeKind::Dark);
        tokens.pill_active_bg = Color32::from_rgb(56, 210, 122);
        assert_eq!(icon_button_style(&tokens, false).selected.icon, Color32::from_rgb(20, 20, 20));
        tokens.pill_active_bg = Color32::from_rgb(35, 45, 70);
        assert_eq!(icon_button_style(&tokens, false).selected.icon, Color32::WHITE);
    }

    #[test]
    fn tokens_kinds_match_filmcraft_palette_brightness_hierarchy() {
        let dark = Tokens::for_kind(ThemeKind::Dark);
        let medium = Tokens::for_kind(ThemeKind::Medium);
        let light = Tokens::for_kind(ThemeKind::Light);

        let luminance = |c: Color32| u16::from(c.r()) + u16::from(c.g()) + u16::from(c.b());

        assert!(luminance(dark.panel_bg) < luminance(medium.panel_bg));
        assert!(luminance(medium.panel_bg) < luminance(light.panel_bg));
        assert!(luminance(dark.app_bg) < luminance(medium.app_bg));
        assert!(luminance(medium.app_bg) < luminance(light.app_bg));

        // Active panel focus outline is always crisp and visible
        assert_eq!(dark.focus, Color32::from_rgb(0x57, 0x94, 0xec));
        assert_eq!(dark.tab_h, 32.0);
        assert_eq!(dark.radius, 0.0);
        assert_eq!(dark.radius_sm, 4.0);
        assert_eq!(dark.gap, 4.0);
    }

    #[test]
    fn craft_ui_widget_palette_maps_correctly_from_tokens() {
        let dark = Tokens::for_kind(ThemeKind::Dark);
        let palette = dark.widget_palette();

        assert_eq!(palette.noninteractive, dark.panel_bg);
        assert_eq!(palette.inactive, dark.field_bg);
        assert_eq!(palette.hovered, dark.hover);
        assert_eq!(palette.active, dark.pressed);
        assert_eq!(palette.open, dark.hover);
    }

    #[test]
    fn appearance_override_tints_focus_and_playhead_consistently() {
        let dark = Tokens::for_kind(ThemeKind::Dark);
        let orange_accent = [255, 128, 0];
        let adapted = dark.with_appearance(Some(orange_accent), false);

        assert_eq!(adapted.accent, Color32::from_rgb(255, 128, 0));
        assert_ne!(adapted.focus, dark.focus);
        assert_eq!(adapted.playhead, adapted.focus);
    }

    #[test]
    fn dock_style_inherits_filmcraft_panel_and_focus_tokens() {
        let dark = Tokens::for_kind(ThemeKind::Dark);
        let mut dock_style = craft_ui::docking::DockStyle::default();
        apply_dock_style(&mut dock_style, &dark);

        assert_eq!(dock_style.tab_height, 32.0);
        assert_eq!(dock_style.tab_background, dark.tab_bg);
        assert_eq!(dock_style.accent, dark.focus);
    }

    #[test]
    fn craft_ui_adapters_inherit_filmcraft_tokens() {
        let dark = Tokens::for_kind(ThemeKind::Dark);

        let btn_style = dark.icon_button_style();
        assert_eq!(btn_style.hovered.fill, dark.hover);
        assert_eq!(btn_style.pressed.unwrap().fill, dark.pressed);
        assert_eq!(btn_style.selected.fill, dark.pill_active_bg);
        assert_eq!(btn_style.focus_stroke.color, dark.focus);

        let chrome = dark.chrome_style();
        assert_eq!(chrome.background, dark.panel_bg);
        assert_eq!(chrome.title_background, dark.tab_bg);
        assert_eq!(chrome.accent, dark.accent);

        let resize = dark.resize_handle_style();
        assert_eq!(resize.hovered, dark.focus);
        assert_eq!(resize.dragged, dark.accent_hover);

        let dock = dark.craft_dock_style();
        assert_eq!(dock.tab_height, 32.0);
        assert_eq!(dock.background, dark.panel_bg);
        assert_eq!(dock.tab_background, dark.tab_bg);
    }

    #[test]
    fn build_visuals_applies_filmcraft_widget_palette_and_focus() {
        let dark = Tokens::for_kind(ThemeKind::Dark);
        let visuals = build_visuals(&dark);

        assert_eq!(visuals.panel_fill, dark.panel_bg);
        assert_eq!(visuals.extreme_bg_color, dark.field_bg);
        assert_eq!(visuals.widgets.inactive.bg_stroke.color, dark.field_border);
        assert_eq!(visuals.widgets.hovered.bg_stroke.color, dark.field_border);
        assert_eq!(visuals.widgets.hovered.bg_fill, dark.hover);
        assert_eq!(visuals.widgets.active.bg_fill, dark.pressed);
        assert_eq!(visuals.selection.bg_fill, dark.accent);
    }

    #[test]
    fn icon_button_style_applies_filmcraft_pill_and_hover_tokens() {
        let dark = Tokens::for_kind(ThemeKind::Dark);
        let style = icon_button_style(&dark, false);

        assert_eq!(style.normal.fill, egui::Color32::TRANSPARENT);
        assert_eq!(style.hovered.fill, dark.hover);
        assert_eq!(style.pressed.map(|p| p.fill), Some(dark.pressed));
        assert_eq!(style.selected.fill, dark.pill_active_bg);
        assert_eq!(style.selected.stroke.color, dark.accent);
        assert_eq!(style.focus_stroke.color, dark.focus);

        let framed = icon_button_style(&dark, true);
        assert_eq!(framed.normal.fill, dark.panel_bg);
        assert_eq!(framed.normal.stroke.color, dark.field_border);
        assert_eq!(framed.hovered.stroke.color, dark.field_border);
    }

    #[test]
    fn icon_button_helpers_render_with_filmcraft_metrics() {
        let ctx = egui::Context::default();
        let config = crate::config::AppConfig::default();
        crate::ui::configure_native_visuals(&ctx, &config);

        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let res = icon_button_glyph(ui, "Test Mute", "M", egui::vec2(28.0, 28.0), false, true);
            assert_eq!(res.rect.size(), egui::vec2(28.0, 28.0));

            let res_selected = icon_button_glyph(ui, "Test Mute Active", "M", egui::vec2(24.0, 24.0), true, false);
            assert_eq!(res_selected.rect.size(), egui::vec2(24.0, 24.0));
        });
        output.textures_delta.clear();
    }

    #[test]
    fn ruler_steps_computes_filmcraft_tick_and_label_ladder() {
        // High zoom: 20 pixels per frame at 30 fps
        let (minor, label) = ruler_steps(20.0, 30);
        assert!(minor >= 1);
        assert!(label >= minor);
        assert_eq!(label % minor, 0);
        assert!(label as f64 * 20.0 >= 110.0);
        assert!(minor as f64 * 20.0 >= 12.0);

        // Medium zoom: 1 pixel per frame at 30 fps (30 px / sec)
        let (minor, label) = ruler_steps(1.0, 30);
        assert_eq!(label % minor, 0);
        assert!(label as f64 * 1.0 >= 110.0);
        assert!(minor as f64 * 1.0 >= 12.0);

        // Low zoom: 0.05 pixels per frame at 30 fps (1.5 px / sec)
        let (minor, label) = ruler_steps(0.05, 30);
        assert_eq!(label % minor, 0);
        assert!(label as f64 * 0.05 >= 110.0);
        assert!(minor as f64 * 0.05 >= 12.0);
    }

    #[test]
    fn zoom_thumb_part_resolves_handles_middle_and_beside() {
        let thumb = egui::Rect::from_min_max(egui::pos2(100.0, 10.0), egui::pos2(200.0, 30.0));

        // Outside vertically or far horizontally
        assert_eq!(ZoomThumbPart::at(thumb, egui::pos2(50.0, 20.0)), ZoomThumbPart::Beside);
        assert_eq!(ZoomThumbPart::at(thumb, egui::pos2(250.0, 20.0)), ZoomThumbPart::Beside);

        // Near left edge (within 9 pt)
        assert_eq!(ZoomThumbPart::at(thumb, egui::pos2(103.0, 20.0)), ZoomThumbPart::Left);

        // Near right edge (within 9 pt)
        assert_eq!(ZoomThumbPart::at(thumb, egui::pos2(196.0, 20.0)), ZoomThumbPart::Right);

        // In the middle
        assert_eq!(ZoomThumbPart::at(thumb, egui::pos2(150.0, 20.0)), ZoomThumbPart::Middle);
    }

    #[test]
    fn zoom_thumb_travel_and_width_bounds() {
        let bar = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(500.0, 20.0));
        let (left, w, per_point) = zoom_thumb(bar, 60.0, 0.0, 100.0);
        assert_eq!(left, 0.0);
        assert!(w >= MIN_THUMB_W);
        assert!(w <= 500.0);
        assert!(per_point >= 0.0);
    }

    #[test]
    fn marker_glyph_and_dropdown_text_render_with_craft_tokens() {
        let ctx = egui::Context::default();
        let config = crate::config::AppConfig::default();
        crate::ui::configure_native_visuals(&ctx, &config);
        let tokens = current(&ctx);

        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let rect = egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(70.0, 24.0));
            let resp = dropdown_text(ui, rect, "Fit", &tokens, egui::Id::unique("test_fit"));
            assert_eq!(resp.rect, rect);

            let marker_rect = egui::Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(7.0, 10.0));
            marker_glyph(ui.painter(), marker_rect, 3.0, MarkerPart::Whole, egui::Color32::from_rgb(0x38, 0x8e, 0x3c));
            marker_glyph(ui.painter(), marker_rect, 3.0, MarkerPart::Left, egui::Color32::from_rgb(0xc2, 0x18, 0x5b));
            marker_glyph(ui.painter(), marker_rect, 3.0, MarkerPart::Right, egui::Color32::from_rgb(0x00, 0x97, 0xa7));
        });
        output.textures_delta.clear();
    }
}
