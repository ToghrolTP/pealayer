use crate::config::AppLanguage;
use eframe::egui;
use std::sync::Arc;

pub(crate) fn configure_ui_fonts(context: &egui::Context, prefer_vazirmatn: bool) {
    let mut fonts = egui::FontDefinitions::default();
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

    // Prefer the platform UI face. Vazirmatn remains immediately behind it so
    // Persian and Arabic text has a bundled, release-safe fallback. For a
    // Persian UI the bundled font is first so shaping is consistent across OSes.
    let system_font = [
        #[cfg(target_os = "windows")]
        r"C:\Windows\Fonts\segoeui.ttf",
        #[cfg(target_os = "macos")]
        "/System/Library/Fonts/SFNS.ttf",
        #[cfg(target_os = "linux")]
        "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
        #[cfg(target_os = "linux")]
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ]
    .iter()
    .find_map(|path| std::fs::read(path).ok());

    fonts.font_data.insert(
        "pealayer-vazirmatn".to_owned(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../../assets/fonts/Vazirmatn-Regular.ttf"
        ))),
    );
    if let Some(bytes) = system_font {
        fonts.font_data.insert(
            "pealayer-system-ui".to_owned(),
            Arc::new(egui::FontData::from_owned(bytes)),
        );
    }

    let has_system_font = fonts.font_data.contains_key("pealayer-system-ui");
    let proportional = fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .expect("egui provides a proportional font family");
    if prefer_vazirmatn {
        proportional.insert(0, "pealayer-vazirmatn".to_owned());
        if has_system_font {
            proportional.insert(1, "pealayer-system-ui".to_owned());
        }
    } else if has_system_font {
        proportional.insert(0, "pealayer-system-ui".to_owned());
        proportional.insert(1, "pealayer-vazirmatn".to_owned());
    } else {
        proportional.insert(0, "pealayer-vazirmatn".to_owned());
    }

    context.set_fonts(fonts);
}

/// Translate application-owned copy. Values learned from media metadata or
/// PCController are deliberately never passed through this table.
pub fn tr(language: AppLanguage, english: &'static str) -> String {
    if language != AppLanguage::Persian {
        return english.to_owned();
    }
    let translated = match english {
        "File" => "پرونده",
        "Edit" => "ویرایش",
        "Audio" => "صدا",
        "Subtitles" => "زیرنویس",
        "Workspace" => "فضای کاری",
        "Workspace tabs" => "زبانه‌های فضای کاری",
        "Right-click to show or hide workspace tabs" => {
            "برای نمایش یا پنهان‌کردن زبانه‌ها کلیک راست کنید"
        }
        "Window" => "پنجره",
        "Panels" => "پنل‌ها",
        "Reset Workspace to Default" => "بازنشانی فضای کاری به حالت پیش‌فرض",
        "All workspace panels are closed" => "همه پنل‌های فضای کاری بسته شده‌اند",
        "Open panels from the Window menu above, or reset the workspace." => {
            "پنل‌ها را از منوی پنجره در بالا باز کنید یا فضای کاری را بازنشانی نمایید."
        }
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
        "High-performance media playback powered by libmpv2, glow OpenGL, and egui." => {
            "پخش پرسرعت رسانه با libmpv2،‏ OpenGL و egui."
        }
        "Featuring real-time 4D haptic timeline synchronization and multi-track relay control." => {
            "با همگام‌سازی بلادرنگ خط زمانی چهاربعدی و کنترل چندترکی رله‌ها."
        }
        "Connect" => "اتصال",
        "Connecting…" => "در حال اتصال…",
        "Disconnect" => "قطع اتصال",
        "Hardware Disconnected" => "سخت‌افزار قطع است",
        "PCController coordinator connected" => "هماهنگ‌کنندهٔ PCController متصل است",
        "Custom endpoint or hardware path" => "نشانی سفارشی یا مسیر سخت‌افزار",
        "Enter a PCController endpoint or an OS hardware path." => {
            "نشانی PCController یا مسیر سخت‌افزار سیستم‌عامل را وارد کنید."
        }
        "Enter a hardware endpoint before connecting." => {
            "پیش از اتصال، نشانی سخت‌افزار را وارد کنید."
        }
        "pccontroller://host:port, tcp://host:port, or direct:<device>" => {
            "pccontroller://میزبان:درگاه، tcp://میزبان:درگاه یا direct:<دستگاه>"
        }
        "RESET E-STOP" => "بازنشانی توقف اضطراری",
        "E-STOP" => "توقف اضطراری",
        "E-STOP ACTIVE" => "توقف اضطراری فعال است",
        "Program Monitor" => "نمایش برنامه",
        "Effect Controls" => "کنترل جلوه‌ها",
        "Identity" => "هویت",
        "Timing constraints" => "محدودیت‌های زمانی",
        "Start time:" => "زمان آغاز:",
        "Duration:" => "مدت:",
        "Hardware target" => "هدف سخت‌افزاری",
        "Connect PCController to choose an available output." => {
            "برای انتخاب خروجی در دسترس به PCController متصل شوید."
        }
        "Target output:" => "خروجی هدف:",
        "Bulk effect controls" => "کنترل گروهی جلوه‌ها",
        "Bulk hardware target override" => "تغییر گروهی هدف سخت‌افزاری",
        "Bulk actions" => "عملیات گروهی",
        "Tracks" => "ترک‌ها",
        "Effects Library" => "کتابخانهٔ جلوه‌ها",
        "Hardware Monitor" => "پایش سخت‌افزار",
        "Timeline" => "خط زمانی",
        "Hardware Monitor Dashboard" => "داشبورد پایش سخت‌افزار",
        "Collapse group" => "جمع‌کردن گروه",
        "Expand group" => "بازکردن گروه",
        "New effect in this group" => "جلوهٔ جدید در این گروه",
        "Edit effect" => "ویرایش جلوه",
        "My effects" => "جلوه‌های من",
        "Macro" => "ماکرو",
        "Strip" => "نوار نور",
        "New" => "جدید",
        "Duplicate" => "تکثیر",
        "Description" => "توضیحات",
        "Manage effects" => "مدیریت جلوه‌ها",
        "Board information" => "اطلاعات برد",
        "Overview" => "نمای کلی",
        "Capabilities" => "قابلیت‌ها",
        "Front panel" => "پنل جلویی",
        "Board settings" => "تنظیمات برد",
        "Board name" => "نام برد",
        "Rename board" => "تغییر نام برد",
        "Board kind" => "نوع برد",
        "Build hash" => "شناسهٔ ساخت",
        "Build timestamp" => "زمان ساخت",
        "Profile" => "پروفایل",
        "Profile revision" => "بازبینی پروفایل",
        "Mode" => "حالت",
        "Not advertised" => "اعلام نشده",
        "Stored by the attached board and read back through PCController." => {
            "در برد متصل ذخیره می‌شود و از طریق PCController بازخوانی می‌شود."
        }
        "Board commands" => "فرمان‌های برد",
        "Reboot board" => "راه‌اندازی دوبارهٔ برد",
        "Confirm board reboot" => "تأیید راه‌اندازی دوبارهٔ برد",
        "Advertised capabilities" => "قابلیت‌های اعلام‌شده",
        "PWM / MOSFET" => "PWM / ماسفت",
        "RF transmit" => "ارسال رادیویی",
        "Addressable strip" => "نوار نور آدرس‌پذیر",
        "Seven-segment display" => "نمایشگر هفت‌بخشی",
        "LCD display" => "نمایشگر LCD",
        "Board macros" => "ماکروهای برد",
        "Capability bits" => "بیت‌های قابلیت",
        "Controls" => "کنترل‌ها",
        "Relays" => "رله‌ها",
        "PWM channels" => "کانال‌های PWM",
        "Peripherals" => "تجهیزات جانبی",
        "Macros" => "ماکروها",
        "Strip renderers" => "پردازشگرهای نوار نور",
        "Live front-panel state" => "وضعیت زندهٔ پنل جلویی",
        "The attached board does not advertise front-panel state." => {
            "برد متصل وضعیت پنل جلویی را اعلام نمی‌کند."
        }
        "The attached board does not advertise settings." => "برد متصل تنظیمات را اعلام نمی‌کند.",
        "Raw segments" => "بخش‌های خام",
        "Brightness" => "روشنایی",
        "Blink" => "چشمک",
        "Pressed keys" => "کلیدهای فشرده",
        "Menu page" => "صفحهٔ منو",
        "Program mode" => "حالت برنامه",
        "LCD available" => "LCD در دسترس",
        "LCD address" => "نشانی LCD",
        "LCD line 1" => "خط اول LCD",
        "LCD line 2" => "خط دوم LCD",
        "Silent mode" => "حالت بی‌صدا",
        "Light mode" => "حالت نور",
        "On brightness" => "روشنایی حالت روشن",
        "Off brightness" => "روشنایی حالت خاموش",
        "Display brightness" => "روشنایی نمایشگر",
        "Status brightness" => "روشنایی وضعیت",
        "Output persistence" => "ماندگاری خروجی",
        "Stream period" => "دورهٔ جریان",
        "Default page" => "صفحهٔ پیش‌فرض",
        "Motion break" => "وقفهٔ حرکت",
        "Persisted" => "ذخیره‌شده",
        "Yes" => "بله",
        "No" => "خیر",
        "Interface" => "رابط",
        "Position" => "موقعیت",
        "Player controls" => "کنترل‌های پخش‌کننده",
        "Connection" => "اتصال",
        "Motion controls" => "کنترل‌های حرکتی",
        "Button behavior" => "رفتار دکمه",
        "Toggle on press" => "تغییر حالت با فشردن",
        "Run only while held" => "حرکت فقط هنگام نگه‌داشتن",
        "Use one-row compact hardware controls" => "استفاده از کنترل سخت‌افزار فشردهٔ یک‌ردیفه",
        "Toggle mode keeps a direction active until another action is chosen. Hold mode sends Stop when the pressed direction is released." => {
            "در حالت تغییر وضعیت، جهت تا انتخاب فرمان بعدی فعال می‌ماند. در حالت نگه‌داشتن، با رهاکردن دکمه فرمان توقف فرستاده می‌شود."
        }
        "Video surface" => "سطح ویدئو",
        "Status bar" => "نوار وضعیت",
        "Hide" => "پنهان‌کردن",
        "Hardware connection" => "اتصال سخت‌افزار",
        "Hardware telemetry" => "دورسنجی سخت‌افزار",
        "Physical status RGB" => "چراغ RGB فیزیکی",
        "Hardware warnings" => "هشدارهای سخت‌افزار",
        "Clock" => "ساعت",
        "Media rate" => "نرخ رسانه",
        "Media frame rate" => "نرخ قاب رسانه",
        "Telemetry" => "دورسنجی",
        "Workspace mode" => "حالت فضای کاری",
        "Search" => "جست‌وجو",
        "Search effects..." => "جست‌وجوی جلوه‌ها…",
        "Connect to PCController to discover live board controls." => {
            "برای دریافت کنترل‌های زندهٔ برد به PCController متصل شوید."
        }
        "PCController is connected; requesting the board capability catalog…" => {
            "PCController متصل است؛ فهرست قابلیت‌های برد در حال درخواست است…"
        }
        "Board connection failed" => "اتصال برد ناموفق بود",
        "PCController detected hardware on" => "PCController سخت‌افزار را در این مسیر شناسایی کرد:",
        "but the board is not responding. Check its USB cable, power, and operating-system device status." => {
            "اما برد پاسخ نمی‌دهد. کابل USB، برق و وضعیت دستگاه در سیستم‌عامل را بررسی کنید."
        }
        "PCController is reachable, but no board is connected. Connect the board and check its USB cable, power, and port." => {
            "PCController در دسترس است، اما هیچ بردی متصل نیست. برد را متصل کنید و کابل USB، برق و درگاه آن را بررسی کنید."
        }
        "PCController connected; board not connected" => "PCController متصل است؛ برد متصل نیست",
        "The connected board advertises no relay controls." => {
            "برد متصل هیچ کنترل رله‌ای اعلام نکرده است."
        }
        "Connected board" => "برد متصل",
        "Group" => "گروه",
        "No group" => "بدون گروه",
        "Change group" => "تغییر گروه",
        "Connected board:" => "برد متصل:",
        "Force ON" => "روشن‌کردن اجباری",
        "ON" => "روشن",
        "OFF" => "خاموش",
        "Release" => "رهاسازی",
        "Override requested" => "درخواست بازنویسی ارسال شد",
        "Board reports ON" => "برد حالت روشن را گزارش می‌کند",
        "Board reports OFF" => "برد حالت خاموش را گزارش می‌کند",
        "Live board control" => "کنترل زندهٔ برد",
        "PCController macro catalog" => "فهرست ماکروهای PCController",
        "Record hardware effect" => "ضبط جلوهٔ سخت‌افزاری",
        "Seat motion take" => "برداشت حرکت صندلی",
        "Start board recording" => "آغاز ضبط از برد",
        "Anchor at the current video time and capture board-applied actions from every PCController surface." => {
            "ضبط را به زمان کنونی ویدئو متصل کنید و فرمان‌های اعمال‌شدهٔ برد را از همهٔ رابط‌های PCController بگیرید."
        }
        "Refresh status" => "تازه‌سازی وضعیت",
        "Save and place" => "ذخیره و جای‌گذاری",
        "Discard" => "دور انداختن",
        "Timeline anchor:" => "نقطهٔ خط زمانی:",
        "Addressable strip effects" => "جلوه‌های نوار نور آدرس‌پذیر",
        "Stop preview" => "توقف پیش‌نمایش",
        "Preview" => "پیش‌نمایش",
        "No compatible relay effects are advertised by the connected PCController." => {
            "PCController متصل هیچ جلوهٔ رله‌ای سازگاری اعلام نکرده است."
        }
        "No effects are advertised by the connected PCController." => {
            "PCController متصل هیچ جلوه‌ای اعلام نکرده است."
        }
        "Connection problem" => "مشکل اتصال",
        "Playback problem" => "مشکل پخش",
        "Operation failed" => "عملیات ناموفق بود",
        "Pealayer could not complete the requested operation." => {
            "پی‌لیر نتوانست عملیات درخواستی را کامل کند."
        }
        "Technical details" => "جزئیات فنی",
        "Copy details" => "رونوشت جزئیات",
        "Close" => "بستن",
        "Open Location / URL" => "باز کردن نشانی یا پیوند",
        "Enter direct video URL, HTTP/HTTPS stream, or HLS link:" => {
            "نشانی مستقیم ویدئو، جریان HTTP/HTTPS یا پیوند HLS را وارد کنید:"
        }
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
        "4D Cinema Editor" => "ویرایشگر سینمای چهاربعدی",
        "Active output status" => "وضعیت خروجی‌های فعال",
        "Effect templates" => "الگوهای جلوه",
        "Icon" => "نماد",
        "Name" => "نام",
        "Duration" => "مدت",
        "Actions" => "عملیات",
        "Add to timeline" => "افزودن به خط زمانی",
        "Create custom template" => "ساخت الگوی سفارشی",
        "Name:" => "نام:",
        "Icon:" => "نماد:",
        "Duration (ms):" => "مدت (میلی‌ثانیه):",
        "Output:" => "خروجی:",
        "No advertised output" => "هیچ خروجی‌ای اعلام نشده است",
        "Create" => "ساختن",
        "Timeline instances" => "نمونه‌های خط زمانی",
        "Start time" => "زمان آغاز",
        "Effect" => "جلوه",
        "Unavailable project effect" => "جلوهٔ پروژه در دسترس نیست",
        "Delete" => "حذف",
        "No Cue Selected" => "هیچ نشانه‌ای انتخاب نشده است",
        "Open media or connect PCController to populate the timeline." => {
            "برای ساخت خط زمانی، رسانه‌ای باز کنید یا به PCController متصل شوید."
        }
        "No media or advertised hardware tracks are available." => {
            "هیچ رسانه یا ترک سخت‌افزاری اعلام‌شده‌ای در دسترس نیست."
        }
        "Replay" => "پخش دوباره",
        "Play" => "پخش",
        "Pause" => "مکث",
        "Stop" => "توقف",
        "Fullscreen" => "تمام‌صفحه",
        "Exit Fullscreen" => "خروج از تمام‌صفحه",
        "Controls Pinned" => "کنترل‌ها ثابت شدند",
        "Controls Unpinned" => "کنترل‌ها از حالت ثابت خارج شدند",
        "Pin Controls" => "ثابت کردن کنترل‌ها",
        "Unpin Controls" => "آزاد کردن کنترل‌ها",
        "Mute" => "قطع صدا",
        "Unmute" => "وصل صدا",
        "Video Files" => "پرونده‌های ویدئویی",
        "Unknown" => "ناشناخته",
        "Drop video file here to play" => "پروندهٔ ویدئو را برای پخش اینجا رها کنید",
        "Invalid format. Use HH:MM:SS.cs (e.g., 00:01:23.45)" => {
            "قالب نامعتبر است. از HH:MM:SS.cs استفاده کنید (برای نمونه 00:01:23.45)"
        }
        "Set to current playback position" => "تنظیم روی موقعیت کنونی پخش",
        "Seek video to this event" => "رفتن به زمان این رویداد در ویدئو",
        "Nudge back by" => "جابجایی به عقب به‌اندازهٔ",
        "Nudge forward by" => "جابجایی به جلو به‌اندازهٔ",
        "Hold Shift for 100ms, Ctrl for 1s" => "Shift برای ۱۰۰ms و Ctrl برای ۱s",
        "Dur" => "مدت",
        "End" => "پایان",
        "Unavailable output" => "خروجی در دسترس نیست",
        "Unavailable project output" => "خروجی پروژه در دسترس نیست",
        "Delete Cue" => "حذف نشانه",
        "Delete All Selected" => "حذف همهٔ انتخاب‌شده‌ها",
        "EMERGENCY STOP ACTIVE - ALL HARDWARE OUTPUTS DISABLED" => {
            "توقف اضطراری فعال است — همهٔ خروجی‌های سخت‌افزاری غیرفعال‌اند"
        }
        "Linear" => "خطی",
        "Smooth (Hermite)" => "نرم (هرمیت)",
        "Step" => "پله‌ای",
        "Delete Keyframe" => "حذف فریم کلیدی",
        "Video" => "ویدئو",
        "Volume" => "صدا",
        "Interpolation" => "درون‌یابی",
        "Time" => "زمان",
        "Value" => "مقدار",
        "Analog Track" => "ترک آنالوگ",
        "Port/Channel" => "درگاه/کانال",
        "Mute Track (M)\nMutes physical output during playback." => {
            "قطع ترک (M)\nخروجی فیزیکی را هنگام پخش غیرفعال می‌کند."
        }
        "Mute Track (M)\nMutes relay physical output during playback." => {
            "قطع ترک (M)\nخروجی فیزیکی رله را هنگام پخش غیرفعال می‌کند."
        }
        "Mute Track (M)\nMutes actuator physical output during playback." => {
            "قطع ترک (M)\nخروجی فیزیکی محرک را هنگام پخش غیرفعال می‌کند."
        }
        "Solo Track (S)\nSolos this relay track output during playback." => {
            "تک‌نوازی ترک (S)\nفقط خروجی این رله را هنگام پخش فعال نگه می‌دارد."
        }
        "Solo Track (S)\nSolos this output during playback." => {
            "تک‌نوازی ترک (S)\nفقط این خروجی را هنگام پخش فعال نگه می‌دارد."
        }
        "Lock Track (L)\nPrevents moving or modifying effects on this track." => {
            "قفل ترک (L)\nجابجایی یا ویرایش جلوه‌های این ترک را متوقف می‌کند."
        }
        "Record Arm (R)\nArms this track for real-time motion capture gesture recording." => {
            "آماده‌سازی ضبط (R)\nاین ترک را برای ضبط بلادرنگ حرکت آماده می‌کند."
        }
        "Live Actuator Fader\nControl actuator intensity in real time (0% - 100%)." => {
            "فیدر زندهٔ محرک\nشدت محرک را بلادرنگ کنترل می‌کند (۰ تا ۱۰۰٪)."
        }
        "Add Keyframe\nInserts a keyframe at the current playhead position." => {
            "افزودن فریم کلیدی\nیک فریم کلیدی در موقعیت کنونی نشانگر درج می‌کند."
        }
        "Timeline Ruler\nClick or drag to scrub playhead. Ctrl+Scroll to zoom time." => {
            "خط‌کش خط زمانی\nبرای پیمایش کلیک یا درگ کنید؛ برای بزرگ‌نمایی Ctrl+اسکرول کنید."
        }
        _ => english,
    };
    visual_text(language, translated)
}

/// Egui's built-in painter positions Unicode scalar values without complex
/// Arabic shaping or the Unicode bidirectional algorithm. Convert application
/// copy to presentation forms and visual order at the UI boundary while
/// retaining logical Unicode in source, configuration, and protocol data.
pub fn visual_text(language: AppLanguage, logical: &str) -> String {
    let contains_arabic_script = logical.chars().any(|character| {
        matches!(
            character,
            '\u{0600}'..='\u{06FF}'
                | '\u{0750}'..='\u{077F}'
                | '\u{08A0}'..='\u{08FF}'
                | '\u{FB50}'..='\u{FDFF}'
                | '\u{FE70}'..='\u{FEFF}'
        )
    });
    if language != AppLanguage::Persian && !contains_arabic_script {
        return logical.to_owned();
    }

    let shaped = ar_reshaper::reshape_line(logical);
    let bidi = unicode_bidi::BidiInfo::new(&shaped, None);
    let mut visual = String::with_capacity(shaped.len());
    for (index, paragraph) in bidi.paragraphs.iter().enumerate() {
        if index > 0 {
            visual.push('\n');
        }
        visual.push_str(&bidi.reorder_line(paragraph, paragraph.range.clone()));
    }
    visual
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
        let persian = tr(AppLanguage::Persian, "Connect");
        assert_ne!(persian, "اتصال");
        assert!(persian.chars().any(|character| {
            ('\u{FB50}'..='\u{FDFF}').contains(&character)
                || ('\u{FE70}'..='\u{FEFF}').contains(&character)
        }));
        assert_eq!(tr(AppLanguage::English, "Connect"), "Connect");
        assert_eq!(tr(AppLanguage::Persian, "Cinema Relay A"), "Cinema Relay A");
    }

    #[test]
    fn persian_copy_is_shaped_and_reordered_for_egui() {
        let logical = "سلام دنیا".to_string();
        let visual = visual_text(AppLanguage::Persian, &logical);
        assert_eq!(logical, "سلام دنیا");
        assert_eq!(visual, "ﺎﯿﻧﺩ ﻡﻼﺳ");
    }

    #[test]
    fn dynamic_persian_name_is_shaped_even_in_english_ui() {
        let logical = "PCController رلهٔ‌اصلی ۱۲۳".to_string();
        let visual = visual_text(AppLanguage::English, &logical);
        assert_eq!(logical, "PCController رلهٔ‌اصلی ۱۲۳");
        assert_ne!(visual, logical);
        assert!(visual.contains("PCController"));
        assert!(visual.chars().any(|character| {
            ('\u{FB50}'..='\u{FDFF}').contains(&character)
                || ('\u{FE70}'..='\u{FEFF}').contains(&character)
        }));
        assert_eq!(
            visual_text(AppLanguage::English, "PCController Relay A"),
            "PCController Relay A"
        );
    }

    #[test]
    fn bundled_vazirmatn_covers_shaped_persian_presentation_forms() {
        let font = include_bytes!("../../assets/fonts/Vazirmatn-Regular.ttf");
        let face = ttf_parser::Face::parse(font, 0).expect("bundled Vazirmatn must parse");
        for glyph in visual_text(AppLanguage::Persian, "سلام دنیا PCController ۱۲۳")
            .chars()
            .filter(|character| !character.is_whitespace() && !character.is_ascii())
        {
            assert!(
                face.glyph_index(glyph).is_some(),
                "font lacks shaped glyph {glyph:?}"
            );
        }
    }
}
