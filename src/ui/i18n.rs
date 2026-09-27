use crate::config::AppLanguage;
use eframe::egui;

/// Translate application-owned copy. Values learned from media metadata or
/// PCController are deliberately never passed through this table.
pub fn tr(language: AppLanguage, english: &'static str) -> &'static str {
    if language != AppLanguage::Persian {
        return english;
    }
    match english {
        "File" => "پرونده",
        "Edit" => "ویرایش",
        "Audio" => "صدا",
        "Subtitles" => "زیرنویس",
        "Workspace" => "فضای کاری",
        "Help" => "راهنما",
        "Language" => "زبان",
        "System language" => "زبان سیستم",
        "English" => "انگلیسی",
        "Persian" => "فارسی",
        "Direction" => "جهت",
        "Automatic" => "خودکار",
        "Left to right" => "چپ به راست",
        "Right to left" => "راست به چپ",
        "Open Video File..." => "باز کردن پروندهٔ ویدئو…",
        "Open Location / URL..." => "باز کردن نشانی یا پیوند…",
        "Open Recent" => "پرونده‌های اخیر",
        "No recent media" => "رسانهٔ اخیری وجود ندارد",
        "Clear Recent" => "پاک کردن فهرست اخیر",
        "Close Video" => "بستن ویدئو",
        "Open Timeline Project..." => "باز کردن پروژهٔ خط زمانی…",
        "Save Timeline (Sidecar)" => "ذخیرهٔ خط زمانی کنار رسانه",
        "Save Timeline As..." => "ذخیرهٔ خط زمانی با نام…",
        "Register as Default Media Player..." => "تنظیم به‌عنوان پخش‌کنندهٔ پیش‌فرض…",
        "Quit" => "خروج",
        "Undo" => "واگرد",
        "Redo" => "ازنو",
        "Audio Track" => "ترک صوتی",
        "None" => "هیچ‌کدام",
        "Audio Settings..." => "تنظیمات صدا…",
        "Audio Settings" => "تنظیمات صدا",
        "Subtitle Track" => "ترک زیرنویس",
        "Enable Subtitles" => "فعال‌سازی زیرنویس",
        "Subtitle Settings..." => "تنظیمات زیرنویس…",
        "Subtitle Settings" => "تنظیمات زیرنویس",
        "NLE Layout (Docked)" => "چیدمان تدوین",
        "Simple Player" => "پخش‌کنندهٔ ساده",
        "Keyboard Shortcuts..." => "میان‌برهای صفحه‌کلید…",
        "About" => "درباره",
        "Modern 4D Video & Haptic Player" => "پخش‌کنندهٔ نوین ویدئوی چهاربعدی و بازخورد لمسی",
        "High-performance media playback powered by libmpv2, glow OpenGL, and egui." =>
            "پخش پرسرعت رسانه با libmpv2،‏ OpenGL و egui.",
        "Featuring real-time 4D haptic timeline synchronization and multi-track relay control." =>
            "با همگام‌سازی بلادرنگ خط زمانی چهاربعدی و کنترل چندترکی رله‌ها.",
        "Connect" => "اتصال",
        "Connecting…" => "در حال اتصال…",
        "Disconnect" => "قطع اتصال",
        "Hardware Disconnected" => "سخت‌افزار قطع است",
        "PCController coordinator connected" => "هماهنگ‌کنندهٔ PCController متصل است",
        "RESET E-STOP" => "بازنشانی توقف اضطراری",
        "E-STOP" => "توقف اضطراری",
        "E-STOP ACTIVE" => "توقف اضطراری فعال است",
        "Program Monitor" => "نمایش برنامه",
        "Effect Controls" => "کنترل جلوه‌ها",
        "Effects Library" => "کتابخانهٔ جلوه‌ها",
        "Hardware Monitor" => "پایش سخت‌افزار",
        "Timeline" => "خط زمانی",
        "Hardware Monitor Dashboard" => "داشبورد پایش سخت‌افزار",
        "Search" => "جست‌وجو",
        "Search effects..." => "جست‌وجوی جلوه‌ها…",
        "Connect to PCController to discover live board controls." =>
            "برای دریافت کنترل‌های زندهٔ برد به PCController متصل شوید.",
        "PCController is connected; waiting for its capability catalog…" =>
            "PCController متصل است؛ در انتظار فهرست قابلیت‌ها…",
        "PCController is reachable, but no board is currently advertising live controls." =>
            "PCController در دسترس است، اما هیچ بردی کنترل زنده اعلام نمی‌کند.",
        "The connected board advertises no relay controls." =>
            "برد متصل هیچ کنترل رله‌ای اعلام نکرده است.",
        "Connected board" => "برد متصل",
        "Force ON" => "روشن‌کردن اجباری",
        "Release" => "رهاسازی",
        "Override requested" => "درخواست بازنویسی ارسال شد",
        "Board reports ON" => "برد حالت روشن را گزارش می‌کند",
        "Board reports OFF" => "برد حالت خاموش را گزارش می‌کند",
        "PCController macro catalog" => "فهرست ماکروهای PCController",
        "No compatible relay effects are advertised by the connected PCController." =>
            "PCController متصل هیچ جلوهٔ رله‌ای سازگاری اعلام نکرده است.",
        "Connection problem" => "مشکل اتصال",
        "Playback problem" => "مشکل پخش",
        "Operation failed" => "عملیات ناموفق بود",
        "Pealayer could not complete the requested operation." =>
            "پی‌لیر نتوانست عملیات درخواستی را کامل کند.",
        "Technical details" => "جزئیات فنی",
        "Copy details" => "رونوشت جزئیات",
        "Close" => "بستن",
        "Open Location / URL" => "باز کردن نشانی یا پیوند",
        "Enter direct video URL, HTTP/HTTPS stream, or HLS link:" =>
            "نشانی مستقیم ویدئو، جریان HTTP/HTTPS یا پیوند HLS را وارد کنید:",
        "Paste" => "چسباندن",
        "Open" => "باز کردن",
        "Cancel" => "انصراف",
        "Keyboard Shortcuts & Controls" => "میان‌برها و کنترل‌های صفحه‌کلید",
        "Shortcut" => "میان‌بر",
        "Action" => "عملکرد",
        "Play / Pause video" => "پخش یا مکث ویدئو",
        "Toggle Fullscreen mode" => "تغییر حالت تمام‌صفحه",
        "Toggle Audio Mute" => "قطع یا وصل صدا",
        "Seek -5s / +5s" => "پیمایش ۵ ثانیه عقب یا جلو",
        "Volume -5% / +5%" => "کاهش یا افزایش ۵ درصدی صدا",
        "Frame Step Forward (+1 frame)" => "یک قاب جلو",
        "Frame Step Backward (-1 frame)" => "یک قاب عقب",
        "Adjust Volume on player/bar" => "تنظیم صدا روی پخش‌کننده",
        "Seek forward / backward" => "پیمایش جلو یا عقب",
        "Toggle Fullscreen / Open Video" => "تمام‌صفحه یا باز کردن ویدئو",
        "Open Player Context Menu" => "باز کردن منوی زمینه‌ای پخش‌کننده",
        "Drop media file onto window to play" => "رسانه را برای پخش روی پنجره رها کنید",
        "Track:" => "ترک:",
        "Appearance" => "ظاهر",
        "Font Size:" => "اندازهٔ قلم:",
        "Synchronization" => "همگام‌سازی",
        "Delay (s):" => "تأخیر (ثانیه):",
        "Reset" => "بازنشانی",
        "Load External Subtitle..." => "بارگیری زیرنویس بیرونی…",
        "Load External Audio..." => "بارگیری صدای بیرونی…",
        "Workspace: NLE Layout" => "فضای کاری: تدوین",
        "Workspace: Simple Player" => "فضای کاری: پخش‌کنندهٔ ساده",
        _ => english,
    }
}

pub fn layout(rtl: bool, main_align: egui::Align) -> egui::Layout {
    if rtl {
        egui::Layout::right_to_left(main_align)
    } else {
        egui::Layout::left_to_right(main_align)
    }
}

pub fn vertical_layout(rtl: bool) -> egui::Layout {
    egui::Layout::top_down(if rtl {
        egui::Align::Max
    } else {
        egui::Align::Min
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translates_owned_copy_but_leaves_dynamic_names_untouched() {
        assert_eq!(tr(AppLanguage::Persian, "Connect"), "اتصال");
        assert_eq!(tr(AppLanguage::English, "Connect"), "Connect");
        assert_eq!(tr(AppLanguage::Persian, "Cinema Relay A"), "Cinema Relay A");
    }
}
