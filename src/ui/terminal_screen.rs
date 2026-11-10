//! Native cell-grid view of the VT parser. No process, clipboard OSC, or transport
//! side effects: the caller owns the parser, input delivery and PTY lifecycle.
use eframe::egui;

// One bracketed paste stays below the backend's 32 KiB atomic input limit.
const MAX_PASTE_BYTES: usize = 16 * 1024;
const MAX_COPY_BYTES: usize = 64 * 1024;
const MAX_SCROLLBACK: usize = 10_000;

#[derive(Default)]
pub struct ScreenViewState {
    scrollback: usize,
    selection: Option<(usize, usize)>,
    selection_anchor: Option<usize>,
    wheel_fraction: f32,
    rejected_paste: bool,
    last_mouse: Option<(usize, bool)>,
}

pub struct ScreenOutput {
    pub rows: u16,
    pub cols: u16,
    pub input: Vec<Vec<u8>>,
    pub focused: bool,
    pub clicked: bool,
    pub clear: bool,
    pub preferences: bool,
}

fn dimensions(size: egui::Vec2, cell: egui::Vec2) -> (u16, u16) {
    let rows = (size.y / cell.y.max(1.0)).floor().clamp(4.0, 80.0) as u16;
    let cols = (size.x / cell.x.max(1.0)).floor().clamp(20.0, 240.0) as u16;
    (rows, cols)
}

fn owns_input(ui: &egui::Ui) -> bool {
    if !ui.is_enabled() || egui::Popup::is_any_open(ui.ctx()) {
        return false;
    }
    let layer = ui.layer_id();
    ui.ctx().memory(|memory| {
        if !memory.is_above_modal_layer(layer) {
            return false;
        }
        let areas = memory.areas();
        let root = areas.parent_layer(layer).unwrap_or(layer);
        let visible = areas.visible_layer_ids();
        let top_window = memory
            .layer_ids()
            .filter(|candidate| {
                matches!(
                    candidate.order,
                    egui::Order::Middle | egui::Order::Foreground
                ) && visible.contains(candidate)
                    && areas.parent_layer(*candidate).is_none()
            })
            .last();
        top_window.is_none_or(|top| root == top || root.order > top.order)
    })
}

fn ansi_color(color: vt100::Color, default: egui::Color32, dark: bool) -> egui::Color32 {
    use egui::Color32;
    match color {
        vt100::Color::Default => default,
        vt100::Color::Rgb(r, g, b) => Color32::from_rgb(r, g, b),
        vt100::Color::Idx(index) => {
            // Conventional ANSI roles; the first sixteen keep contrast against
            // the application's live light/dark terminal surface.
            let normal = if dark {
                [
                    [25, 28, 34],
                    [225, 91, 100],
                    [115, 194, 132],
                    [224, 190, 101],
                    [112, 157, 231],
                    [188, 139, 218],
                    [97, 188, 198],
                    [218, 222, 228],
                    [112, 121, 136],
                    [249, 131, 139],
                    [149, 220, 162],
                    [250, 219, 131],
                    [151, 186, 250],
                    [218, 172, 245],
                    [137, 218, 228],
                    [248, 249, 251],
                ]
            } else {
                [
                    [30, 33, 40],
                    [166, 40, 48],
                    [40, 119, 61],
                    [136, 103, 19],
                    [41, 91, 174],
                    [129, 63, 158],
                    [29, 112, 122],
                    [194, 200, 208],
                    [98, 107, 122],
                    [196, 52, 63],
                    [42, 137, 67],
                    [155, 116, 20],
                    [54, 109, 199],
                    [154, 81, 183],
                    [33, 136, 147],
                    [250, 250, 252],
                ]
            };
            let [r, g, b] = if index < 16 {
                normal[index as usize]
            } else if index < 232 {
                let value = index - 16;
                let levels = [0, 95, 135, 175, 215, 255];
                [
                    levels[(value / 36) as usize],
                    levels[((value / 6) % 6) as usize],
                    levels[(value % 6) as usize],
                ]
            } else {
                let level = 8 + (index - 232) * 10;
                [level, level, level]
            };
            Color32::from_rgb(r, g, b)
        }
    }
}
pub(crate) fn web_color(color: vt100::Color) -> Option<String> {
    if color == vt100::Color::Default {
        return None;
    }
    let c = ansi_color(color, egui::Color32::WHITE, true);
    Some(format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b()))
}

fn key_bytes(
    key: egui::Key,
    modifiers: egui::Modifiers,
    application_cursor: bool,
) -> Option<Vec<u8>> {
    use egui::Key;
    if modifiers.ctrl {
        let name = key.name();
        if name.len() == 1 && name.as_bytes()[0].is_ascii_alphabetic() {
            let value = name.as_bytes()[0].to_ascii_uppercase() - b'A' + 1;
            return Some(if modifiers.alt {
                vec![27, value]
            } else {
                vec![value]
            });
        }
        let control = match key {
            Key::Space | Key::Num2 => Some(0),
            Key::OpenBracket => Some(27),
            Key::Backslash => Some(28),
            Key::CloseBracket => Some(29),
            Key::Num6 => Some(30),
            Key::Minus => Some(31),
            _ => None,
        };
        if let Some(value) = control {
            return Some(vec![value]);
        }
    }
    let modifier =
        1 + u8::from(modifiers.shift) + 2 * u8::from(modifiers.alt) + 4 * u8::from(modifiers.ctrl);
    let cursor = match key {
        Key::ArrowUp => Some('A'),
        Key::ArrowDown => Some('B'),
        Key::ArrowRight => Some('C'),
        Key::ArrowLeft => Some('D'),
        Key::Home => Some('H'),
        Key::End => Some('F'),
        _ => None,
    };
    if let Some(suffix) = cursor {
        return Some(
            if modifier > 1 {
                format!("\x1b[1;{modifier}{suffix}")
            } else if application_cursor {
                format!("\x1bO{suffix}")
            } else {
                format!("\x1b[{suffix}")
            }
            .into_bytes(),
        );
    }
    let function = match key {
        Key::F1 => Some('P'),
        Key::F2 => Some('Q'),
        Key::F3 => Some('R'),
        Key::F4 => Some('S'),
        _ => None,
    };
    if let Some(suffix) = function {
        return Some(
            if modifier > 1 {
                format!("\x1b[1;{modifier}{suffix}")
            } else {
                format!("\x1bO{suffix}")
            }
            .into_bytes(),
        );
    }
    let tilde = match key {
        Key::Insert => Some(2),
        Key::Delete => Some(3),
        Key::PageUp => Some(5),
        Key::PageDown => Some(6),
        Key::F5 => Some(15),
        Key::F6 => Some(17),
        Key::F7 => Some(18),
        Key::F8 => Some(19),
        Key::F9 => Some(20),
        Key::F10 => Some(21),
        Key::F11 => Some(23),
        Key::F12 => Some(24),
        _ => None,
    };
    if let Some(code) = tilde {
        return Some(
            if modifier > 1 {
                format!("\x1b[{code};{modifier}~")
            } else {
                format!("\x1b[{code}~")
            }
            .into_bytes(),
        );
    }
    let mut bytes = match key {
        Key::Enter => vec![b'\r'],
        Key::Backspace => vec![127],
        Key::Tab if modifiers.shift => b"\x1b[Z".to_vec(),
        Key::Tab => vec![b'\t'],
        Key::Escape => vec![27],
        _ => return None,
    };
    if modifiers.alt {
        bytes.insert(0, 27);
    }
    Some(bytes)
}

fn paste_bytes(text: &str, bracketed: bool) -> Option<Vec<u8>> {
    // Reject oversized pastes atomically, never execute a truncated command.
    if text.len() > MAX_PASTE_BYTES
        || (!bracketed && text.contains(['\r', '\n']))
        || text
            .chars()
            .any(|ch| ch.is_control() && !matches!(ch, '\r' | '\n' | '\t'))
    {
        return None;
    }
    let mut result = Vec::with_capacity(text.len() + 12);
    if bracketed {
        result.extend_from_slice(b"\x1b[200~");
    }
    result.extend_from_slice(text.as_bytes());
    if bracketed {
        result.extend_from_slice(b"\x1b[201~");
    }
    Some(result)
}

fn selected_text(screen: &vt100::Screen, selection: Option<(usize, usize)>) -> String {
    let (rows, cols) = screen.size();
    let Some((anchor, end)) = selection else {
        return String::new();
    };
    let mut start = anchor.min(end).min(usize::from(rows) * usize::from(cols));
    let end = anchor.max(end).min(usize::from(rows) * usize::from(cols));
    if start < end
        && start % usize::from(cols) != 0
        && screen
            .cell(
                (start / usize::from(cols)) as u16,
                (start % usize::from(cols)) as u16,
            )
            .is_some_and(|cell| cell.is_wide_continuation())
    {
        start -= 1;
    }
    let mut result = String::new();
    let mut line = String::new();
    for index in start..end {
        let row = (index / usize::from(cols)) as u16;
        let col = (index % usize::from(cols)) as u16;
        if col == 0 && index != start {
            result.push_str(line.trim_end());
            result.push('\n');
            line.clear();
        }
        if let Some(cell) = screen.cell(row, col) {
            if cell.is_wide_continuation() {
                continue;
            }
            let text = cell.contents();
            if text.is_empty() {
                line.push(' ');
            } else {
                line.push_str(&text);
            }
        }
        if result.len() + line.len() >= MAX_COPY_BYTES {
            break;
        }
    }
    result.push_str(line.trim_end());
    result
}

/// Paint a bounded viewport. Consumed input is returned, never delivered here.
/// The caller must also suppress earlier global shortcuts while this Id owns focus.
pub fn show(
    ui: &mut egui::Ui,
    parser: &mut vt100::Parser,
    state: &mut ScreenViewState,
    id: egui::Id,
    interactive: bool,
    font_size: f32,
) -> ScreenOutput {
    show_with_preferences(
        ui,
        parser,
        state,
        id,
        interactive,
        font_size,
        &super::terminal::TerminalPreferences::default(),
    )
}

pub fn show_with_preferences(
    ui: &mut egui::Ui,
    parser: &mut vt100::Parser,
    state: &mut ScreenViewState,
    id: egui::Id,
    interactive: bool,
    font_size: f32,
    preferences: &super::terminal::TerminalPreferences,
) -> ScreenOutput {
    show_impl(
        ui,
        parser,
        state,
        id,
        interactive,
        font_size,
        preferences,
        true,
    )
}
pub fn show_shared(
    ui: &mut egui::Ui,
    parser: &mut vt100::Parser,
    state: &mut ScreenViewState,
    id: egui::Id,
    interactive: bool,
    preferences: &super::terminal::TerminalPreferences,
) -> ScreenOutput {
    show_impl(
        ui,
        parser,
        state,
        id,
        interactive,
        preferences.font_size,
        preferences,
        false,
    )
}
fn show_impl(
    ui: &mut egui::Ui,
    parser: &mut vt100::Parser,
    state: &mut ScreenViewState,
    id: egui::Id,
    interactive: bool,
    font_size: f32,
    preferences: &super::terminal::TerminalPreferences,
    resize: bool,
) -> ScreenOutput {
    let mut font = egui::TextStyle::Monospace.resolve(ui.style());
    if font_size.is_finite() {
        font.size = font_size.clamp(8.0, 40.0);
    }
    let cell = ui.fonts_mut(|fonts| {
        egui::vec2(
            fonts.glyph_width(&font, 'M').max(1.0),
            fonts.row_height(&font).ceil().max(1.0),
        )
    });
    let available = ui.available_size().max(egui::vec2(1.0, 1.0));
    let (rows, cols) = if resize {
        dimensions(available, cell)
    } else {
        parser.screen().size()
    };
    if parser.screen().size() != (rows, cols) {
        parser.set_size(rows, cols);
        state.selection = None;
    }
    let (rect, _) = ui.allocate_exact_size(available, egui::Sense::hover());
    let response = ui
        .interact(rect, id, egui::Sense::click_and_drag())
        .on_hover_cursor(egui::CursorIcon::Text);
    let active_layer = owns_input(ui);
    let clicked = response.clicked() && active_layer;
    if clicked && interactive {
        response.request_focus();
    }
    let focused = interactive && response.has_focus() && active_layer;
    if focused {
        ui.memory_mut(|memory| {
            memory.set_focus_lock_filter(
                id,
                egui::EventFilter {
                    tab: true,
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    escape: true,
                    ..Default::default()
                },
            )
        });
    }
    let point_index = |point: egui::Pos2| {
        let row = ((point.y - rect.top()) / cell.y)
            .floor()
            .clamp(0.0, f32::from(rows - 1)) as usize;
        let col = ((point.x - rect.left()) / cell.x)
            .floor()
            .clamp(0.0, f32::from(cols - 1)) as usize;
        row * usize::from(cols) + col
    };
    let modifiers = ui.input(|input| input.modifiers);
    let mouse = interactive
        && active_layer
        && !modifiers.shift
        && parser.screen().alternate_screen()
        && parser.screen().mouse_protocol_mode() != vt100::MouseProtocolMode::None
        && parser.screen().mouse_protocol_encoding() == vt100::MouseProtocolEncoding::Sgr;
    if active_layer && !mouse {
        if response.drag_started() {
            if let Some(point) = ui.input(|input| input.pointer.press_origin()) {
                let index = point_index(point);
                state.selection_anchor = Some(index);
                state.selection = Some((index, index + 1));
            }
        }
        if response.dragged() {
            if let (Some(anchor), Some(point)) =
                (state.selection_anchor, response.interact_pointer_pos())
            {
                let end = point_index(point);
                state.selection = Some((anchor.min(end), anchor.max(end) + 1));
            }
        } else if clicked {
            state.selection = None;
        }
    }

    let mut copy = false;
    let mut select_all = false;
    let mut input_bytes = Vec::new();
    if mouse && let Some(point) = ui.input(|input| input.pointer.interact_pos()) {
        if rect.contains(point) || response.dragged() {
            let index = point_index(point);
            let col = index % usize::from(cols) + 1;
            let row = index / usize::from(cols) + 1;
            let (pressed, released, down) = ui.input(|input| {
                (
                    input.pointer.button_pressed(egui::PointerButton::Primary),
                    input.pointer.button_released(egui::PointerButton::Primary),
                    input.pointer.primary_down(),
                )
            });
            if pressed {
                input_bytes.push(format!("\x1b[<0;{col};{row}M").into_bytes());
            }
            if released {
                input_bytes.push(format!("\x1b[<0;{col};{row}m").into_bytes());
            }
            let (right_pressed, right_released) = ui.input(|input| {
                (
                    input.pointer.button_pressed(egui::PointerButton::Secondary),
                    input
                        .pointer
                        .button_released(egui::PointerButton::Secondary),
                )
            });
            if right_pressed {
                input_bytes.push(format!("\x1b[<2;{col};{row}M").into_bytes());
            }
            if right_released {
                input_bytes.push(format!("\x1b[<2;{col};{row}m").into_bytes());
            }
            if down
                && !pressed
                && state.last_mouse != Some((index, true))
                && matches!(
                    parser.screen().mouse_protocol_mode(),
                    vt100::MouseProtocolMode::ButtonMotion | vt100::MouseProtocolMode::AnyMotion
                )
            {
                input_bytes.push(format!("\x1b[<32;{col};{row}M").into_bytes());
            }
            state.last_mouse = Some((index, down));
        }
    } else {
        state.last_mouse = None;
    }
    let mut paste_rejected = false;
    let mut scroll_lines = 0_i32;
    let hover = response.hovered() && active_layer;
    let events = ui.input_mut(|input| {
        let all = std::mem::take(&mut input.events);
        let mut claimed = Vec::new();
        for event in all {
            let take = (focused
                && matches!(
                    event,
                    egui::Event::Text(_)
                        | egui::Event::Paste(_)
                        | egui::Event::Copy
                        | egui::Event::Cut
                        | egui::Event::Key { .. }
                        | egui::Event::Ime(_)
                ))
                || (hover && matches!(event, egui::Event::MouseWheel { .. }));
            if take {
                claimed.push(event);
            } else {
                input.events.push(event);
            }
        }
        if hover {
            input.smooth_scroll_delta = egui::Vec2::ZERO;
        }
        claimed
    });
    let has_paste = events
        .iter()
        .any(|event| matches!(event, egui::Event::Paste(_)));
    let has_copy = events
        .iter()
        .any(|event| matches!(event, egui::Event::Copy));
    let has_cut = events.iter().any(|event| matches!(event, egui::Event::Cut));
    let application_cursor = parser.screen().application_cursor();
    let bracketed = parser.screen().bracketed_paste();
    for event in events {
        match event {
            egui::Event::MouseWheel { unit, delta, .. } => {
                state.wheel_fraction += match unit {
                    egui::MouseWheelUnit::Line => delta.y,
                    egui::MouseWheelUnit::Page => delta.y * f32::from(rows),
                    egui::MouseWheelUnit::Point => delta.y / cell.y,
                };
                let whole = state.wheel_fraction.trunc() as i32;
                if mouse {
                    if let Some(point) = ui.input(|input| input.pointer.hover_pos()) {
                        let index = point_index(point);
                        let col = index % usize::from(cols) + 1;
                        let row = index / usize::from(cols) + 1;
                        for _ in 0..whole.unsigned_abs().min(32) {
                            input_bytes.push(
                                format!("\x1b[<{};{col};{row}M", if whole > 0 { 64 } else { 65 })
                                    .into_bytes(),
                            );
                        }
                    }
                } else {
                    scroll_lines += whole;
                }
                state.wheel_fraction -= whole as f32;
            }
            egui::Event::Copy => {
                if state.selection.is_some() || modifiers.shift {
                    copy = true;
                    select_all |= state.selection.is_none();
                } else if modifiers.ctrl {
                    // Some native integrations replace Ctrl-C with Copy rather
                    // than also emitting Key: preserve terminal interrupt.
                    input_bytes.push(vec![3]);
                }
            }
            egui::Event::Cut => {
                if state.selection.is_some() {
                    copy = true;
                } else if modifiers.ctrl {
                    input_bytes.push(vec![24]);
                }
            }
            egui::Event::Paste(text) => match paste_bytes(&text, bracketed) {
                Some(bytes) => input_bytes.push(bytes),
                None => paste_rejected = true,
            },
            egui::Event::Text(text) | egui::Event::Ime(egui::ImeEvent::Commit(text)) => {
                if text.len() <= MAX_PASTE_BYTES {
                    let mut bytes = text.into_bytes();
                    if modifiers.alt {
                        bytes.insert(0, 27);
                    }
                    input_bytes.push(bytes);
                }
            }
            egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => {
                if (modifiers.ctrl || modifiers.command)
                    && ((has_copy && key == egui::Key::C) || (has_cut && key == egui::Key::X))
                {
                    // The corresponding native clipboard event is handled once.
                } else if (modifiers.ctrl || modifiers.command)
                    && modifiers.shift
                    && key == egui::Key::A
                {
                    select_all = true;
                } else if (modifiers.ctrl || modifiers.command)
                    && matches!(key, egui::Key::C | egui::Key::X)
                    && (state.selection.is_some() || modifiers.shift)
                {
                    copy = true;
                    if state.selection.is_none() {
                        select_all = true;
                    }
                } else if has_paste && (modifiers.ctrl || modifiers.command) && key == egui::Key::V
                {
                    // Paste has its own OS event. Do not send a second Ctrl-V.
                } else if modifiers.shift && matches!(key, egui::Key::PageUp | egui::Key::PageDown)
                {
                    scroll_lines += if key == egui::Key::PageUp {
                        i32::from(rows)
                    } else {
                        -i32::from(rows)
                    };
                } else if let Some(bytes) = key_bytes(key, modifiers, application_cursor) {
                    input_bytes.push(bytes);
                }
            }
            _ => {}
        }
    }
    if scroll_lines != 0 {
        state.scrollback = (state.scrollback as i64 + i64::from(scroll_lines))
            .clamp(0, MAX_SCROLLBACK as i64) as usize;
        state.selection = None;
    }
    if !input_bytes.is_empty() {
        state.scrollback = 0;
        state.selection = None;
        state.rejected_paste = false;
    }
    if paste_rejected {
        state.rejected_paste = true;
    }
    parser.set_scrollback(state.scrollback);
    state.scrollback = parser.screen().scrollback();
    let mut clear = false;
    let mut preferences_requested = false;
    if !mouse {
        response.context_menu(|ui| {
            if ui
                .add_enabled(
                    state.selection.is_some(),
                    egui::Button::new(format!("{}  Copy selection", super::icons::COPY)),
                )
                .clicked()
            {
                copy = true;
                ui.close();
            }
            if ui
                .button(format!(
                    "{}  Select all visible text",
                    super::icons::LIST_CHECKS
                ))
                .clicked()
            {
                select_all = true;
                ui.close();
            }
            if ui
                .add_enabled(
                    interactive,
                    egui::Button::new(format!("{}  Paste", super::icons::CLIPBOARD)),
                )
                .clicked()
            {
                if let Ok(text) =
                    arboard::Clipboard::new().and_then(|mut clipboard| clipboard.get_text())
                {
                    if let Some(bytes) = paste_bytes(&text, bracketed) {
                        input_bytes.push(bytes);
                    } else {
                        state.rejected_paste = true;
                    }
                }
                response.request_focus();
                ui.close();
            }
            if ui
                .add_enabled(
                    state.scrollback != 0,
                    egui::Button::new(format!("{}  Scroll to latest", super::icons::ARROW_DOWN)),
                )
                .clicked()
            {
                state.scrollback = 0;
                state.selection = None;
                parser.set_scrollback(0);
                ui.close();
            }
            ui.separator();
            if ui
                .button(format!("{}  Clear terminal", super::icons::TRASH))
                .clicked()
            {
                clear = true;
                ui.close();
            }
            if ui
                .button(format!("{}  Terminal preferences", super::icons::GEAR))
                .clicked()
            {
                preferences_requested = true;
                ui.close();
            }
        });
    }
    if select_all {
        state.selection = Some((0, usize::from(rows) * usize::from(cols)));
    }
    if copy {
        ui.ctx()
            .copy_text(selected_text(parser.screen(), state.selection));
    }

    let screen = parser.screen();
    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    let background = ui.visuals().extreme_bg_color;
    let foreground = ui.visuals().text_color();
    painter.rect_filled(rect, 0.0, background);
    let mut bold_font = font.clone();
    let bold_family = egui::FontFamily::Name("pealayer-status-mono-strong".into());
    if ui.fonts_mut(|fonts| fonts.families().contains(&bold_family)) {
        bold_font.family = bold_family;
    }
    let selected = state.selection.map(|(a, b)| (a.min(b), a.max(b)));
    for row in 0..rows {
        for col in 0..cols {
            let Some(content) = screen.cell(row, col) else {
                continue;
            };
            if content.is_wide_continuation() {
                continue;
            }
            let width = if content.is_wide() { 2.0 } else { 1.0 };
            let cell_rect = egui::Rect::from_min_size(
                rect.min + egui::vec2(f32::from(col) * cell.x, f32::from(row) * cell.y),
                egui::vec2(cell.x * width, cell.y),
            );
            let mut fg = ansi_color(content.fgcolor(), foreground, ui.visuals().dark_mode);
            // PortableShell's default semantic canvas follows the live app
            // surface; explicit non-role ANSI/truecolor values stay untouched.
            if content.fgcolor() == vt100::Color::Rgb(230, 233, 239) {
                fg = foreground;
            }
            if content.fgcolor() == vt100::Color::Rgb(151, 160, 175) {
                fg = ui.visuals().weak_text_color();
            }
            if !ui.visuals().dark_mode {
                fg = match content.fgcolor() {
                    vt100::Color::Rgb(126, 224, 163) | vt100::Color::Rgb(166, 218, 149) => {
                        egui::Color32::from_rgb(23, 120, 64)
                    }
                    vt100::Color::Rgb(255, 204, 102) => ui.visuals().warn_fg_color,
                    vt100::Color::Rgb(255, 107, 122) => ui.visuals().error_fg_color,
                    vt100::Color::Rgb(255, 184, 108) => egui::Color32::from_rgb(160, 80, 16),
                    vt100::Color::Rgb(112, 211, 235) => egui::Color32::from_rgb(0, 100, 125),
                    _ => fg,
                };
            }
            // Command/prompt role is violet in the freestanding core. Remap
            // that role only; warnings/errors/numbers retain semantic colors.
            if preferences.accent_commands
                && matches!(
                    content.fgcolor(),
                    vt100::Color::Rgb(216, 160, 255)
                        | vt100::Color::Idx(183)
                        | vt100::Color::Idx(141)
                        | vt100::Color::Idx(5)
                )
            {
                fg = ui.visuals().selection.stroke.color;
            }
            let mut bg = ansi_color(content.bgcolor(), background, ui.visuals().dark_mode);
            if content.bgcolor() == vt100::Color::Rgb(24, 26, 31) {
                bg = background;
            }
            if content.inverse() {
                std::mem::swap(&mut fg, &mut bg);
            }
            let index = usize::from(row) * usize::from(cols) + usize::from(col);
            if selected.is_some_and(|(start, end)| index < end && index + width as usize > start) {
                bg = ui.visuals().selection.bg_fill;
                fg = ui.visuals().selection.stroke.color;
            }
            painter.rect_filled(cell_rect, 0.0, bg);
            let text = content.contents();
            if !text.is_empty() {
                painter
                    .with_clip_rect(cell_rect.intersect(rect).intersect(ui.clip_rect()))
                    .text(
                        cell_rect.left_center(),
                        egui::Align2::LEFT_CENTER,
                        text,
                        if content.bold() {
                            bold_font.clone()
                        } else {
                            font.clone()
                        },
                        fg,
                    );
            }
            if content.underline() {
                painter.line_segment(
                    [
                        cell_rect.left_bottom() - egui::vec2(0.0, 1.0),
                        cell_rect.right_bottom() - egui::vec2(0.0, 1.0),
                    ],
                    egui::Stroke::new(1.0, fg),
                );
            }
        }
    }
    // Blink only the focused, visible terminal. No hidden-session timer and no
    // continuous repaint; reduced-motion policy keeps the caret steady.
    let blink = preferences.cursor_blink && focused && ui.style().animation_time > 0.0;
    let time = ui.input(|input| input.time);
    let caret_visible = !blink || (time / 0.6).floor() as u64 % 2 == 0;
    if interactive && state.scrollback == 0 && !screen.hide_cursor() && blink {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs_f64(
                0.6 - time.rem_euclid(0.6),
            ));
    }
    if interactive && state.scrollback == 0 && !screen.hide_cursor() && caret_visible {
        let (row, col) = screen.cursor_position();
        if row < rows && col < cols {
            let mut cursor = egui::Rect::from_min_size(
                rect.min + egui::vec2(f32::from(col) * cell.x, f32::from(row) * cell.y),
                cell,
            );
            match preferences.cursor_style {
                super::terminal::CursorStyle::Bar => cursor.max.x = cursor.min.x + 2.0,
                super::terminal::CursorStyle::Underline => cursor.min.y = cursor.max.y - 2.0,
                super::terminal::CursorStyle::Block => {}
            }
            painter.rect_stroke(
                cursor,
                0.0,
                egui::Stroke::new(
                    if focused { 2.0 } else { 1.0 },
                    if focused {
                        ui.visuals().selection.stroke.color
                    } else {
                        ui.visuals().weak_text_color()
                    },
                ),
                egui::StrokeKind::Inside,
            );
        }
    }
    if state.rejected_paste {
        response
            .clone()
            .on_hover_text("Paste rejected: maximum 16 KiB, no embedded control codes, and multiline input requires bracketed paste. Nothing was sent.");
        painter.rect_stroke(
            rect,
            0.0,
            egui::Stroke::new(2.0, ui.visuals().error_fg_color),
            egui::StrokeKind::Inside,
        );
        let message = "Paste exceeds 16 KiB; nothing sent";
        let warning_font = egui::FontId::proportional(font.size.min(12.0));
        let error_color = ui.visuals().error_fg_color;
        let galley = ui.fonts_mut(|fonts| {
            fonts.layout(
                message.into(),
                warning_font,
                error_color,
                (rect.width() - 16.0).max(1.0),
            )
        });
        let warning_rect = egui::Rect::from_min_size(
            rect.right_bottom() - galley.size() - egui::vec2(12.0, 12.0),
            galley.size() + egui::vec2(8.0, 8.0),
        );
        painter.rect_filled(warning_rect, 3.0, background);
        painter.galley(
            warning_rect.min + egui::vec2(4.0, 4.0),
            galley,
            ui.visuals().error_fg_color,
        );
    }
    let accessible = format!(
        "Terminal, {rows} rows, {cols} columns. {}",
        screen.contents()
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::Role::TextInput, interactive, accessible.clone())
    });
    ScreenOutput {
        rows,
        cols,
        input: input_bytes,
        focused,
        clicked,
        clear,
        preferences: preferences_requested,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_sequences_keep_cursor_modes_and_terminal_control_bytes() {
        assert_eq!(
            key_bytes(egui::Key::ArrowUp, egui::Modifiers::NONE, false).unwrap(),
            b"\x1b[A"
        );
        assert_eq!(
            key_bytes(egui::Key::ArrowUp, egui::Modifiers::NONE, true).unwrap(),
            b"\x1bOA"
        );
        assert_eq!(
            key_bytes(egui::Key::ArrowLeft, egui::Modifiers::CTRL, true).unwrap(),
            b"\x1b[1;5D"
        );
        assert_eq!(
            key_bytes(egui::Key::C, egui::Modifiers::CTRL, false).unwrap(),
            [3]
        );
        assert_eq!(
            key_bytes(egui::Key::Enter, egui::Modifiers::NONE, false).unwrap(),
            b"\r"
        );
        assert_eq!(
            key_bytes(egui::Key::Tab, egui::Modifiers::SHIFT, false).unwrap(),
            b"\x1b[Z"
        );
    }

    #[test]
    fn paste_is_bounded_atomic_and_bracketed_only_when_requested() {
        assert_eq!(
            paste_bytes("a\nb", true).unwrap(),
            b"\x1b[200~a\nb\x1b[201~"
        );
        assert!(paste_bytes("a\nb", false).is_none());
        assert!(paste_bytes("\x1b[201~play\r", true).is_none());
        assert_eq!(paste_bytes("plain text", false).unwrap(), b"plain text");
        assert!(paste_bytes(&"x".repeat(MAX_PASTE_BYTES + 1), true).is_none());
    }

    #[test]
    fn fragmented_ansi_unicode_and_copy_read_real_cells() {
        let mut parser = vt100::Parser::new(4, 20, 100);
        parser.process(b"\x1b[38;2;12;");
        parser.process(b"34;56m");
        let bytes = "界e\u{301}".as_bytes();
        parser.process(&bytes[..2]);
        parser.process(&bytes[2..]);
        let screen = parser.screen();
        assert_eq!(
            screen.cell(0, 0).unwrap().fgcolor(),
            vt100::Color::Rgb(12, 34, 56)
        );
        assert!(screen.cell(0, 0).unwrap().is_wide());
        assert!(screen.cell(0, 1).unwrap().is_wide_continuation());
        assert_eq!(selected_text(screen, Some((0, 3))), "界e\u{301}");
        assert_eq!(selected_text(screen, Some((1, 2))), "界");
    }

    #[test]
    fn viewport_and_ansi_cube_are_bounded() {
        assert_eq!(
            dimensions(egui::vec2(1.0, 1.0), egui::vec2(8.0, 16.0)),
            (4, 20)
        );
        assert_eq!(
            dimensions(egui::vec2(10000.0, 10000.0), egui::vec2(8.0, 16.0)),
            (80, 240)
        );
        assert_eq!(
            ansi_color(vt100::Color::Idx(196), egui::Color32::WHITE, true),
            egui::Color32::RED
        );
        assert_eq!(
            ansi_color(vt100::Color::Idx(255), egui::Color32::WHITE, true),
            egui::Color32::from_gray(238)
        );
    }

    #[test]
    fn focused_grid_claims_space_text_and_navigation_but_readonly_view_does_not() {
        let ctx = egui::Context::default();
        let id = egui::Id::unique("terminal-input-test");
        let mut parser = vt100::Parser::new(4, 20, 100);
        let mut state = ScreenViewState::default();
        let mut frame = ctx.run_ui(egui::RawInput::default(), |ui| {
            show(ui, &mut parser, &mut state, id, true, 12.0);
        });
        // Headless fixtures have no renderer to upload/free the font atlas.
        frame.textures_delta.clear();
        ctx.memory_mut(|memory| memory.request_focus(id));
        let events = vec![
            egui::Event::Key {
                key: egui::Key::Space,
                physical_key: Some(egui::Key::Space),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::Text(" ".into()),
            egui::Event::Key {
                key: egui::Key::ArrowUp,
                physical_key: Some(egui::Key::ArrowUp),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            },
        ];
        let mut frame = ctx.run_ui(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ui| {
                let output = show(ui, &mut parser, &mut state, id, true, 12.0);
                assert!(output.focused);
                assert_eq!(output.input, vec![b" ".to_vec(), b"\x1b[A".to_vec()]);
                assert!(!ui.input(|input| {
                    input.events.iter().any(|event| {
                        matches!(event, egui::Event::Text(_) | egui::Event::Key { .. })
                    })
                }));
            },
        );
        frame.textures_delta.clear();
        let mut frame = ctx.run_ui(
            egui::RawInput {
                events: vec![egui::Event::Text("x".into())],
                ..Default::default()
            },
            |ui| {
                let output = show(ui, &mut parser, &mut state, id, false, 12.0);
                assert!(!output.focused);
                assert!(output.input.is_empty());
                assert!(ui.input(|input| {
                    input
                        .events
                        .iter()
                        .any(|event| matches!(event, egui::Event::Text(_)))
                }));
            },
        );
        frame.textures_delta.clear();
    }
}
