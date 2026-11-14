//! VT state plus an independent semantic-foreground mask. No RGB/index value
//! doubles as a role in the actual screen; ordinary cyan/violet stay ordinary.
//! Only embedded/paired PortableShell screens pay for the second bounded grid.
use std::num::NonZeroU16;

fn dimensions(rows: u16, cols: u16) -> (NonZeroU16, NonZeroU16) {
    (
        NonZeroU16::new(rows).unwrap_or(NonZeroU16::MIN),
        NonZeroU16::new(cols).unwrap_or(NonZeroU16::MIN),
    )
}

pub(super) fn screen_size(screen: &vt100::Screen) -> (u16, u16) {
    let (rows, cols) = screen.size();
    (rows.get(), cols.get())
}

pub struct Parser {
    inner: vt100::Parser,
    roles: Option<(vt100::Parser, vte::Parser, bool)>,
}
impl Parser {
    pub fn new(rows: u16, cols: u16, scrollback: usize) -> Self {
        let (rows, cols) = dimensions(rows, cols);
        Self {
            inner: vt100::Parser::new(rows, cols, scrollback),
            roles: None,
        }
    }
    pub fn semantic(rows: u16, cols: u16, scrollback: usize) -> Self {
        let (rows, cols) = dimensions(rows, cols);
        Self {
            inner: vt100::Parser::new(rows, cols, scrollback),
            roles: Some((
                vt100::Parser::new(rows, cols, scrollback),
                vte::Parser::new(),
                false,
            )),
        }
    }
    pub fn process(&mut self, bytes: &[u8]) {
        self.inner.process(bytes);
        if let Some((mask, decoder, pending)) = &mut self.roles {
            // Preserve all cursor, scrolling, erasure, wide-cell, alternate and
            // saved-attribute behavior using the identical VT implementation.
            for byte in bytes {
                mask.process(std::slice::from_ref(byte));
                let mut roles = Roles(None);
                decoder.advance(&mut roles, std::slice::from_ref(byte));
                // vte dispatches OSC at the ESC of ST. Do not inject SGR
                // until its following backslash has finished in both parsers.
                if *pending && *byte != 0x1b {
                    mask.process(b"\x1b[38;5;1m");
                    *pending = false;
                }
                if let Some(accent) = roles.0 {
                    if accent && *byte == 0x1b {
                        *pending = true;
                    } else {
                        mask.process(if accent { b"\x1b[38;5;1m" } else { b"\x1b[39m" });
                    }
                }
            }
        }
    }
    pub fn screen(&self) -> &vt100::Screen {
        self.inner.screen()
    }
    pub fn application_palette(&self) -> bool {
        self.roles.is_some()
    }
    pub fn set_size(&mut self, rows: u16, cols: u16) {
        let (rows, cols) = dimensions(rows, cols);
        self.inner.set_size(rows, cols);
        if let Some((mask, _, _)) = &mut self.roles {
            mask.set_size(rows, cols);
        }
    }
    pub fn set_scrollback(&mut self, lines: usize) {
        self.inner.screen_mut().set_scrollback(lines);
        if let Some((mask, _, _)) = &mut self.roles {
            mask.screen_mut().set_scrollback(lines);
        }
    }
    pub fn accent(&self, row: u16, col: u16) -> bool {
        self.roles
            .as_ref()
            .and_then(|(mask, _, _)| mask.screen().cell(row, col))
            .is_some_and(|cell| cell.fgcolor() == vt100::Color::Idx(1))
    }
}
struct Roles(Option<bool>);
impl vte::Perform for Roles {
    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        if params == [b"777".as_slice(), b"ps", b"ink", b"accent"] {
            self.0 = Some(true);
        }
    }
    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        intermediates: &[u8],
        ignore: bool,
        action: char,
    ) {
        if ignore || !intermediates.is_empty() || action != 'm' {
            return;
        }
        let mut params = params.iter();
        let mut foreground = false;
        while let Some(group) = params.next() {
            let code = group.first().copied().unwrap_or(0);
            foreground |= matches!(code, 0 | 30..=39 | 90..=97);
            // Extended colors consume their own arguments; RGB zeros are not
            // resets. Colon-form components are contained in the same group.
            if matches!(code, 38 | 48 | 58) && group.len() == 1 {
                match params.next().and_then(|group| group.first()).copied() {
                    Some(2) => {
                        for _ in 0..3 {
                            params.next();
                        }
                    }
                    Some(5) => {
                        params.next();
                    }
                    _ => {}
                }
            }
        }
        if foreground {
            self.0 = Some(false);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    const ACCENT: &[u8] = b"\x1b[38;2;216;160;255m\x1b]777;ps;ink;accent\x1b\\";
    #[test]
    fn semantic_accent_is_explicit_chunk_safe_and_not_a_color_heuristic() {
        let mut parser = Parser::semantic(4, 40, 100);
        let mut bytes = ACCENT.to_vec();
        bytes.extend_from_slice(b"a\x1b[48;2;0;30;39mb\x1b[1mc\x1b[36md\x1b[38;2;216;160;255me");
        for byte in bytes {
            parser.process(&[byte]);
        }
        assert_eq!(parser.screen().contents(), "abcde");
        for col in 0..3 {
            assert!(parser.accent(0, col));
        }
        for col in 3..5 {
            assert!(!parser.accent(0, col));
        }
        assert_eq!(
            parser.screen().cell(0, 4).unwrap().fgcolor(),
            vt100::Color::Rgb(216, 160, 255)
        );
    }
    #[test]
    fn foreground_resets_clear_roles_but_background_and_colon_rgb_do_not() {
        for reset in ["0", "39", "31", "97", "38;5;6", "38:2::0:39:0"] {
            let mut parser = Parser::semantic(4, 40, 0);
            parser.process(ACCENT);
            parser.process(format!("a\x1b[{reset}mb").as_bytes());
            assert!(parser.accent(0, 0));
            assert!(!parser.accent(0, 1), "{reset}");
        }
        for keep in [
            "1",
            "4",
            "48;5;0",
            "48:2::0:39:0",
            "48;2;0;0;0",
            "58;2;0;39;31",
        ] {
            let mut parser = Parser::semantic(4, 40, 0);
            parser.process(ACCENT);
            parser.process(format!("a\x1b[{keep}mb").as_bytes());
            assert!(parser.accent(0, 1), "{keep}");
        }
    }
    #[test]
    fn roles_follow_scrollback_erasure_saved_attributes_and_alternate_screen() {
        let mut parser = Parser::semantic(4, 20, 100);
        parser.process(ACCENT);
        parser.process(b"a\x1b7\x1b[39mb\x1b8c\r\nx\r\ny\r\nz\r\nq");
        parser.set_scrollback(1);
        assert!(parser.accent(0, 0));
        assert_eq!(parser.screen().cell(0, 0).unwrap().contents(), "a");
        parser.set_scrollback(0);
        parser.process(b"\x1b[?1049h\x1b[39m\x1b[2J\x1b[Hplain");
        assert!(!parser.accent(0, 0));
        parser.process(b"\x1b[?1049l");
        assert!(parser.accent(0, 0));
        parser.process(b"\x1b[39m\x1b[2J\x1b[H");
        assert!(!parser.accent(0, 0));
        parser.set_size(5, 30);
        assert_eq!(screen_size(parser.screen()), (5, 30));
    }
    #[test]
    fn marker_inside_dcs_or_unknown_osc_is_not_interpreted() {
        let mut parser = Parser::semantic(4, 40, 0);
        parser.process(b"\x1bP777;ps;ink;accent\x1b\\a\x1b]777;other;ink;accent\x1b\\b");
        assert!(!parser.accent(0, 0));
        assert!(!parser.accent(0, 1));
    }
}
