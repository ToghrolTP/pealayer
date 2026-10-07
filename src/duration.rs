/// Format an effect length for operator-facing UI while preserving millisecond
/// precision for sub-second effects. A zero-length streamed effect has no
/// predetermined end and therefore reads as continuous instead of `0 ms`.
pub fn format_effect_duration(duration_ms: u64) -> String {
    if duration_ms == 0 {
        return "Until stopped".to_string();
    }
    if duration_ms < 1_000 {
        return format!("{duration_ms} ms");
    }

    let minutes = duration_ms / 60_000;
    let remainder_ms = duration_ms % 60_000;
    if minutes == 0 {
        return format!("{} sec", seconds_text(remainder_ms));
    }
    if remainder_ms == 0 {
        return format!("{minutes} min");
    }
    format!("{minutes} min {} sec", seconds_text(remainder_ms))
}

/// Formats an editable millisecond value using the shortest lossless operator
/// notation. Unlike [`format_effect_duration`], zero stays `0ms` because this
/// helper is also used by ordinary time fields where zero does not mean an
/// unbounded effect.
pub fn format_time_value_ms(duration_ms: u64) -> String {
    format_time_value_ms_with_preference(duration_ms, true)
}

/// Formats a millisecond-backed editor according to the operator preference.
/// Human mode chooses the shortest lossless unit; raw mode always uses ms.
pub fn format_time_value_ms_with_preference(
    duration_ms: u64,
    human_readable_units: bool,
) -> String {
    if !human_readable_units {
        return format!("{duration_ms}ms");
    }
    if duration_ms < 1_000 {
        return format!("{duration_ms}ms");
    }

    let minutes = duration_ms / 60_000;
    let remainder_ms = duration_ms % 60_000;
    if minutes == 0 {
        return format!("{}s", seconds_text(remainder_ms));
    }
    if remainder_ms == 0 {
        return format!("{minutes}m");
    }
    format!("{minutes}m {}s", seconds_text(remainder_ms))
}

/// Formats microsecond-backed protocol fields without losing sub-millisecond
/// precision. Raw mode still prefers milliseconds, using a decimal when the
/// value cannot be represented by a whole millisecond.
pub fn format_time_value_us_with_preference(
    duration_us: u64,
    human_readable_units: bool,
) -> String {
    if !human_readable_units {
        return format!("{}ms", decimal_units(duration_us, 1_000));
    }
    if duration_us < 1_000 {
        return format!("{duration_us}µs");
    }
    if duration_us < 1_000_000 {
        return format!("{}ms", decimal_units(duration_us, 1_000));
    }
    format!("{}s", decimal_units(duration_us, 1_000_000))
}

/// Compact timeline clock: omit zero leading hours/minutes, but retain zero
/// fields inside the clock (one hour is `1:00:00`, not `1`). Storage stays in ms.
pub fn format_timeline_time_ms(time_ms: u64, show_subseconds: bool) -> String {
    let seconds = time_ms / 1_000;
    let hours = seconds / 3_600;
    let minutes = seconds / 60 % 60;
    let mut clock = if hours > 0 {
        format!("{hours}:{minutes:02}:{:02}", seconds % 60)
    } else if minutes > 0 {
        format!("{minutes}:{:02}", seconds % 60)
    } else {
        (seconds % 60).to_string()
    };
    let milliseconds = time_ms % 1_000;
    if show_subseconds && milliseconds > 0 {
        clock.push_str(&format!(".{milliseconds:03}"));
    }
    clock
}

/// Parses the editable counterpart of [`format_time_value_ms`]. Bare numbers
/// remain milliseconds so pasted values from the living PCController contract
/// retain their exact meaning. Operators may also enter `250ms`, `5s`,
/// `1.25s`, `2m 5s`, or a `MM:SS(.mmm)` clock value.
pub fn parse_time_value_ms(text: &str) -> Option<u64> {
    parse_time_value(text, 1_000.0)
}

/// Parses a time value into exact microseconds. Bare numbers retain the native
/// microsecond meaning of the edited protocol field.
pub fn parse_time_value_us(text: &str) -> Option<u64> {
    parse_time_value(text, 1_000_000.0)
}

fn parse_time_value(text: &str, target_units_per_second: f64) -> Option<u64> {
    let normalized = text
        .trim()
        .to_ascii_lowercase()
        .replace('µ', "u")
        .replace('μ', "u");
    if normalized.is_empty() {
        return None;
    }
    if normalized.contains(':') {
        return parse_clock_value(&normalized, target_units_per_second);
    }

    let compact = normalized.split_whitespace().collect::<String>();
    if let Ok(bare) = compact.parse::<f64>() {
        return rounded_units(bare);
    }

    let mut remainder = compact.as_str();
    let mut seconds_total = 0.0_f64;
    while !remainder.is_empty() {
        let number_end = remainder
            .char_indices()
            .take_while(|(_, character)| character.is_ascii_digit() || *character == '.')
            .map(|(index, character)| index + character.len_utf8())
            .last()?;
        let amount = remainder[..number_end].parse::<f64>().ok()?;
        if !amount.is_finite() || amount < 0.0 {
            return None;
        }
        remainder = &remainder[number_end..];
        let unit_end = remainder
            .char_indices()
            .take_while(|(_, character)| character.is_ascii_alphabetic())
            .map(|(index, character)| index + character.len_utf8())
            .last()?;
        let unit = &remainder[..unit_end];
        let multiplier = match unit {
            "us" | "usec" | "usecs" | "microsecond" | "microseconds" => 0.000_001,
            "ms" | "msec" | "msecs" | "millisecond" | "milliseconds" => 0.001,
            "s" | "sec" | "secs" | "second" | "seconds" => 1.0,
            "m" | "min" | "mins" | "minute" | "minutes" => 60.0,
            "h" | "hr" | "hrs" | "hour" | "hours" => 3_600.0,
            _ => return None,
        };
        seconds_total += amount * multiplier;
        if !seconds_total.is_finite() {
            return None;
        }
        remainder = &remainder[unit_end..];
    }
    rounded_units(seconds_total * target_units_per_second)
}

fn parse_clock_value(text: &str, target_units_per_second: f64) -> Option<u64> {
    let parts = text.split(':').map(str::trim).collect::<Vec<_>>();
    let seconds = match parts.as_slice() {
        [minutes, seconds] => {
            let minutes = minutes.parse::<u64>().ok()? as f64;
            let seconds = seconds.parse::<f64>().ok()?;
            (seconds >= 0.0 && seconds < 60.0).then_some(minutes * 60.0 + seconds)?
        }
        [hours, minutes, seconds] => {
            let hours = hours.parse::<u64>().ok()? as f64;
            let minutes = minutes.parse::<u8>().ok()?;
            let seconds = seconds.parse::<f64>().ok()?;
            (minutes < 60 && seconds >= 0.0 && seconds < 60.0)
                .then_some(hours * 3_600.0 + f64::from(minutes) * 60.0 + seconds)?
        }
        _ => return None,
    };
    rounded_units(seconds * target_units_per_second)
}

fn rounded_units(value: f64) -> Option<u64> {
    (value.is_finite() && value >= 0.0 && value <= u64::MAX as f64)
        .then(|| value.round() as u64)
}

fn decimal_units(value: u64, divisor: u64) -> String {
    let whole = value / divisor;
    let remainder = value % divisor;
    if remainder == 0 {
        return whole.to_string();
    }
    let width = divisor.ilog10() as usize;
    format!("{whole}.{remainder:0width$}")
        .trim_end_matches('0')
        .to_string()
}

/// A shared editor for every millisecond-backed time/duration field. The
/// backing value and protocol remain exact milliseconds while the UI reads and
/// accepts human units.
pub fn time_value_drag<'a>(
    value_ms: &'a mut u64,
    range: std::ops::RangeInclusive<u64>,
    speed_ms: f64,
    human_readable_units: bool,
) -> eframe::egui::DragValue<'a> {
    eframe::egui::DragValue::new(value_ms)
        .range(range)
        .speed(speed_ms)
        // Parse the complete token on Enter or focus loss. Updating after each
        // keystroke would commit the leading `1` before an operator can finish
        // typing `1s`, making the suffix appear to be ignored.
        .update_while_editing(false)
        .custom_formatter(move |value, _| {
            format_time_value_ms_with_preference(
                value.round().max(0.0) as u64,
                human_readable_units,
            )
        })
        .custom_parser(|text| parse_time_value_ms(text).map(|value| value as f64))
}

/// Shared editor for microsecond-backed timing fields such as RF pulse widths
/// and effect timing tolerance.
pub fn time_value_us_drag<'a>(
    value_us: &'a mut u64,
    range: std::ops::RangeInclusive<u64>,
    speed_us: f64,
    human_readable_units: bool,
) -> eframe::egui::DragValue<'a> {
    eframe::egui::DragValue::new(value_us)
        .range(range)
        .speed(speed_us)
        .update_while_editing(false)
        .custom_formatter(move |value, _| {
            format_time_value_us_with_preference(
                value.round().max(0.0) as u64,
                human_readable_units,
            )
        })
        .custom_parser(|text| parse_time_value_us(text).map(|value| value as f64))
}

pub fn format_effect_duration_for_language(
    language: crate::config::AppLanguage,
    duration_ms: u64,
) -> String {
    if !matches!(language, crate::config::AppLanguage::Persian) {
        return format_effect_duration(duration_ms);
    }
    if duration_ms == 0 {
        return "تا زمان توقف".to_string();
    }
    if duration_ms < 1_000 {
        return format!("{duration_ms} میلی‌ثانیه");
    }

    let minutes = duration_ms / 60_000;
    let remainder_ms = duration_ms % 60_000;
    if minutes == 0 {
        return format!("{} ثانیه", seconds_text(remainder_ms));
    }
    if remainder_ms == 0 {
        return format!("{minutes} دقیقه");
    }
    format!("{minutes} دقیقه و {} ثانیه", seconds_text(remainder_ms))
}

fn seconds_text(duration_ms: u64) -> String {
    let whole = duration_ms / 1_000;
    let fractional = duration_ms % 1_000;
    if fractional == 0 {
        return whole.to_string();
    }
    format!("{whole}.{fractional:03}")
        .trim_end_matches('0')
        .to_string()
}

#[cfg(test)]
mod tests {
    #[test]
    fn timeline_clock_strips_only_leading_zero_units() {
        for (milliseconds, expected) in [
            (0, "0"), (5_000, "5"), (59_000, "59"), (60_000, "1:00"),
            (65_000, "1:05"), (3_600_000, "1:00:00"),
            (3_661_000, "1:01:01"), (360_000_000, "100:00:00"),
        ] {
            assert_eq!(super::format_timeline_time_ms(milliseconds, false), expected);
            assert_eq!(super::format_timeline_time_ms(milliseconds, true), expected);
        }
        assert_eq!(super::format_timeline_time_ms(500, true), "0.500");
        assert_eq!(super::format_timeline_time_ms(65_125, true), "1:05.125");
        assert_eq!(super::format_timeline_time_ms(65_999, false), "1:05");
    }
    use super::{
        format_effect_duration, format_effect_duration_for_language, format_time_value_ms,
        format_time_value_ms_with_preference, format_time_value_us_with_preference,
        parse_time_value_ms, parse_time_value_us,
    };

    #[test]
    fn effect_durations_use_the_most_readable_unit_without_losing_precision() {
        assert_eq!(format_effect_duration(0), "Until stopped");
        assert_eq!(format_effect_duration(27), "27 ms");
        assert_eq!(format_effect_duration(999), "999 ms");
        assert_eq!(format_effect_duration(1_000), "1 sec");
        assert_eq!(format_effect_duration(1_452), "1.452 sec");
        assert_eq!(format_effect_duration(12_500), "12.5 sec");
        assert_eq!(format_effect_duration(60_000), "1 min");
        assert_eq!(format_effect_duration(61_250), "1 min 1.25 sec");
        assert_eq!(format_effect_duration(150_000), "2 min 30 sec");
    }

    #[test]
    fn persian_duration_units_are_native_and_rtl_ready() {
        assert_eq!(
            format_effect_duration_for_language(crate::config::AppLanguage::Persian, 0),
            "تا زمان توقف"
        );
        assert_eq!(
            format_effect_duration_for_language(crate::config::AppLanguage::Persian, 61_250),
            "1 دقیقه و 1.25 ثانیه"
        );
    }

    #[test]
    fn editable_time_values_use_compact_human_units() {
        assert_eq!(format_time_value_ms(0), "0ms");
        assert_eq!(format_time_value_ms(250), "250ms");
        assert_eq!(format_time_value_ms(5_000), "5s");
        assert_eq!(format_time_value_ms(61_250), "1m 1.25s");
        assert_eq!(format_time_value_ms(120_000), "2m");
    }

    #[test]
    fn editable_time_values_accept_units_and_exact_milliseconds() {
        assert_eq!(parse_time_value_ms("5000"), Some(5_000));
        assert_eq!(parse_time_value_ms("5000 ms"), Some(5_000));
        assert_eq!(parse_time_value_ms("5s"), Some(5_000));
        assert_eq!(parse_time_value_ms("1.25 s"), Some(1_250));
        assert_eq!(parse_time_value_ms("1m 5s"), Some(65_000));
        assert_eq!(parse_time_value_ms("1 minute 5 seconds"), Some(65_000));
        assert_eq!(parse_time_value_ms("1h 2m 3.5s"), Some(3_723_500));
        assert_eq!(parse_time_value_ms("1:05.250"), Some(65_250));
        assert_eq!(parse_time_value_ms("1:02:03.500"), Some(3_723_500));
        assert_eq!(parse_time_value_ms("1:60"), None);
        assert_eq!(parse_time_value_ms("forever"), None);
    }

    #[test]
    fn raw_time_unit_preference_always_formats_milliseconds() {
        assert_eq!(format_time_value_ms_with_preference(250, false), "250ms");
        assert_eq!(format_time_value_ms_with_preference(5_000, false), "5000ms");
        assert_eq!(format_time_value_ms_with_preference(65_250, false), "65250ms");
    }

    #[test]
    fn microsecond_fields_accept_and_normalize_human_units_losslessly() {
        assert_eq!(parse_time_value_us("350us"), Some(350));
        assert_eq!(parse_time_value_us("350µs"), Some(350));
        assert_eq!(parse_time_value_us("1ms"), Some(1_000));
        assert_eq!(parse_time_value_us("1.25s"), Some(1_250_000));
        assert_eq!(format_time_value_us_with_preference(350, true), "350µs");
        assert_eq!(format_time_value_us_with_preference(1_250, true), "1.25ms");
        assert_eq!(format_time_value_us_with_preference(1_250, false), "1.25ms");
    }
}
