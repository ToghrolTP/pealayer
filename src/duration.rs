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

/// Parses the editable counterpart of [`format_time_value_ms`]. Bare numbers
/// remain milliseconds so pasted values from the living PCController contract
/// retain their exact meaning. Operators may also enter `250ms`, `5s`,
/// `1.25s`, `2m 5s`, or a `MM:SS(.mmm)` clock value.
pub fn parse_time_value_ms(text: &str) -> Option<u64> {
    let normalized = text.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }
    if let Some((minutes, seconds)) = normalized.split_once(':') {
        let minutes = minutes.trim().parse::<u64>().ok()?;
        let seconds = seconds.trim().parse::<f64>().ok()?;
        if !seconds.is_finite() || seconds < 0.0 || seconds >= 60.0 {
            return None;
        }
        return milliseconds_from_parts(minutes, seconds);
    }

    let compact = normalized.split_whitespace().collect::<String>();
    if let Some(milliseconds) = compact.strip_suffix("ms") {
        return milliseconds.parse().ok();
    }
    if let Some(minutes_end) = compact.find('m') {
        let minutes = compact[..minutes_end].parse::<u64>().ok()?;
        let remainder = &compact[minutes_end + 1..];
        if remainder.is_empty() {
            return minutes.checked_mul(60_000);
        }
        let seconds = remainder.strip_suffix('s')?.parse::<f64>().ok()?;
        if !seconds.is_finite() || seconds < 0.0 || seconds >= 60.0 {
            return None;
        }
        return milliseconds_from_parts(minutes, seconds);
    }
    if let Some(seconds) = compact.strip_suffix('s') {
        let seconds = seconds.parse::<f64>().ok()?;
        if !seconds.is_finite() || seconds < 0.0 {
            return None;
        }
        return rounded_milliseconds(seconds);
    }
    compact.parse().ok()
}

fn milliseconds_from_parts(minutes: u64, seconds: f64) -> Option<u64> {
    minutes
        .checked_mul(60_000)?
        .checked_add(rounded_milliseconds(seconds)?)
}

fn rounded_milliseconds(seconds: f64) -> Option<u64> {
    let milliseconds = seconds * 1_000.0;
    (milliseconds.is_finite() && milliseconds >= 0.0 && milliseconds <= u64::MAX as f64)
        .then(|| milliseconds.round() as u64)
}

/// A shared editor for every millisecond-backed time/duration field. The
/// backing value and protocol remain exact milliseconds while the UI reads and
/// accepts human units.
pub fn time_value_drag<'a>(
    value_ms: &'a mut u64,
    range: std::ops::RangeInclusive<u64>,
    speed_ms: f64,
) -> eframe::egui::DragValue<'a> {
    eframe::egui::DragValue::new(value_ms)
        .range(range)
        .speed(speed_ms)
        .custom_formatter(|value, _| format_time_value_ms(value.round().max(0.0) as u64))
        .custom_parser(|text| parse_time_value_ms(text).map(|value| value as f64))
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
    use super::{
        format_effect_duration, format_effect_duration_for_language, format_time_value_ms,
        parse_time_value_ms,
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
        assert_eq!(parse_time_value_ms("1:05.250"), Some(65_250));
        assert_eq!(parse_time_value_ms("1:60"), None);
        assert_eq!(parse_time_value_ms("forever"), None);
    }
}
