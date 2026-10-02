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
    use super::{format_effect_duration, format_effect_duration_for_language};

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
}
