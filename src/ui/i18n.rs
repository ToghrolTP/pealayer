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
        "Active track" => "ترک فعال",
        "Hide subtitles" => "پنهان‌کردن زیرنویس",
        "Show subtitles" => "نمایش زیرنویس",
        "Workspace" => "فضای کاری",
        "Workspace profiles" => "نمایه‌های فضای کاری",
        "Workspace profiles..." => "نمایه‌های فضای کاری…",
        "Workspaces" => "فضاهای کاری",
        "Manage workspaces..." => "مدیریت فضاهای کاری…",
        "Manage workspaces" => "مدیریت فضاهای کاری",
        "Add workspace" => "افزودن فضای کاری",
        "No workspace profiles" => "هیچ فضای کاری وجود ندارد",
        "Active workspace" => "فضای کاری فعال",
        "Delete workspace" => "حذف فضای کاری",
        "Replace with current workspace" => "جایگزینی با فضای کاری کنونی",
        "Switch workspace" => "تغییر فضای کاری",
        "Save this window, panel, dialog, and scroll arrangement as a reusable workspace." => {
            "این چیدمان پنجره، پنل‌ها، گفتگوها و پیمایش را به‌عنوان فضای کاری قابل استفادهٔ مجدد ذخیره کنید."
        }
        "Workspace name" => "نام فضای کاری",
        "No saved workspace profiles" => "نمایهٔ فضای کاری ذخیره‌شده‌ای وجود ندارد",
        "Workspace profile saved" => "نمایهٔ فضای کاری ذخیره شد",
        "Workspace profile restored" => "نمایهٔ فضای کاری بازیابی شد",
        "Workspace profile deleted" => "نمایهٔ فضای کاری حذف شد",
        "Workspace profile not found" => "نمایهٔ فضای کاری پیدا نشد",
        "Enter a workspace profile name" => "نامی برای نمایهٔ فضای کاری وارد کنید",
        "Delete workspace profile" => "حذف نمایهٔ فضای کاری",
        "Restore" => "بازیابی",
        "Active" => "فعال",
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
        "Subtitle layout" => "چیدمان زیرنویس",
        "Global text replacements" => "جایگزینی‌های سراسری متن",
        "Applied in order to every text subtitle" => "به‌ترتیب روی همهٔ زیرنویس‌های متنی اعمال می‌شود",
        "Source text" => "متن مبدأ",
        "Replacement" => "جایگزین",
        "Add replacement" => "افزودن جایگزینی",
        "Remove replacement" => "حذف جایگزینی",
        "Open Video File..." => "باز کردن پروندهٔ ویدئو…",
        "Open Location / URL..." => "باز کردن نشانی یا پیوند…",
        "Open Recent" => "پرونده‌های اخیر",
        "No recent media" => "رسانهٔ اخیری وجود ندارد",
        "Clear Recent" => "پاک کردن فهرست اخیر",
        "Close Video" => "بستن ویدئو",
        "Open Timeline Project..." => "باز کردن پروژهٔ خط زمانی…",
        "Save Timeline (Sidecar)" => "ذخیرهٔ خط زمانی کنار رسانه",
        "Save Timeline As..." => "ذخیرهٔ خط زمانی با نام…",
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
        "E-STOP" => "توقف اضطراری",
        "E-STOP ACTIVE" => "توقف اضطراری فعال است",
        "Emergency stop" => "توقف اضطراری",
        "Hide E-STOP" => "پنهان‌کردن توقف اضطراری",
        "Release E-STOP?" => "توقف اضطراری آزاد شود؟",
        "Release E-STOP" => "آزادکردن توقف اضطراری",
        "Hardware outputs and effects will be allowed again." => {
            "خروجی‌های سخت‌افزار و جلوه‌ها دوباره مجاز خواهند شد."
        }
        "Do not ask again" => "دوباره پرسیده نشود",
        "Show E-STOP in the application header" => "نمایش توقف اضطراری در سربرگ برنامه",
        "Confirm before releasing E-STOP" => "تأیید پیش از آزادکردن توقف اضطراری",
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
        "Hardware control" => "کنترل سخت‌افزار",
        "Manage..." => "مدیریت…",
        "Manage channels" => "مدیریت کانال‌ها",
        "All board channels" => "همهٔ کانال‌های برد",
        "Order" => "ترتیب",
        "Order must be between" => "ترتیب باید بین این مقادیر باشد:",
        "Move up" => "انتقال به بالا",
        "Move down" => "انتقال به پایین",
        "Other controls" => "کنترل‌های دیگر",
        "Rename, control, and reorder every advertised board channel" => {
            "تغییر نام، کنترل و مرتب‌سازی همهٔ کانال‌های اعلام‌شدهٔ برد"
        }
        "Presentation" => "نحوهٔ نمایش",
        "Save presentation" => "ذخیرهٔ نحوهٔ نمایش",
        "Restore defaults" => "بازگردانی پیش‌فرض‌ها",
        "Live control" => "کنترل زنده",
        "Turn on" => "روشن‌کردن",
        "Turn off" => "خاموش‌کردن",
        "Stable key" => "کلید پایدار",
        "This item has no live actions advertised." => {
            "برای این مورد هیچ عملیات زنده‌ای اعلام نشده است."
        }
        "Place at playhead" => "قرار دادن در نشانگر پخش",
        "The Controller effects track is not available." => "ترک جلوه‌های کنترل‌کننده در دسترس نیست.",
        "Manage" => "مدیریت",
        "Jump to cue start" => "پرش به آغاز نشانه",
        "Cue start" => "آغاز نشانه",
        "Run now" => "اجرای فوری",
        "Restore default name" => "بازگردانی نام پیش‌فرض",
        "Type" => "نوع",
        "Control" => "کنترل",
        "Board" => "برد",
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
        "Only settings advertised by the connected board are shown." => {
            "فقط تنظیماتی که برد متصل اعلام کرده است نمایش داده می‌شوند."
        }
        "General" => "عمومی",
        "Door sound cues" => "نشانه‌های صوتی در",
        "Relay sound cues" => "نشانه‌های صوتی رله",
        "Telemetry period" => "دورهٔ تله‌متری",
        "Use 0 to disable periodic telemetry" => {
            "برای غیرفعال‌کردن تله‌متری دوره‌ای مقدار ۰ را وارد کنید"
        }
        "Temperature sensors" => "حسگرهای دما",
        "Swap temperature roles" => "جابجایی نقش حسگرهای دما",
        "Enclosure lighting" => "روشنایی محفظه",
        "Automatic (door)" => "خودکار (در)",
        "Front-panel display" => "نمایشگر پنل جلویی",
        "Open brightness" => "روشنایی در حالت باز",
        "Closed brightness" => "روشنایی در حالت بسته",
        "Remember last page" => "به‌خاطر سپردن آخرین صفحه",
        "Status light" => "چراغ وضعیت",
        "Fallback color" => "رنگ جایگزین",
        "Red" => "قرمز",
        "Blue" => "آبی",
        "Violet" => "بنفش",
        "Green" => "سبز",
        "White" => "سفید",
        "Motion and relays" => "حرکت و رله‌ها",
        "Door policy" => "سیاست در",
        "Always allow motion" => "حرکت همیشه مجاز باشد",
        "Only while door is closed" => "فقط هنگامی که در بسته است",
        "Only while door is open" => "فقط هنگامی که در باز است",
        "Never allow motion" => "حرکت هرگز مجاز نباشد",
        "Exit hold" => "مکث خروج",
        "Relays restored after restart" => "رله‌های بازیابی‌شده پس از راه‌اندازی",
        "Remember motion defaults" => "به‌خاطر سپردن پیش‌فرض‌های حرکت",
        "Remember user relays" => "به‌خاطر سپردن رله‌های کاربر",
        "Remember PWM outputs" => "به‌خاطر سپردن خروجی‌های PWM",
        "Retain motion direction when stopped" => "حفظ جهت حرکت هنگام توقف",
        "Measurements" => "اندازه‌گیری‌ها",
        "Voltage decimals" => "رقم‌های اعشاری ولتاژ",
        "Current decimals" => "رقم‌های اعشاری جریان",
        "Advanced" => "پیشرفته",
        "Programming latch" => "قفل برنامه‌ریزی",
        "Programming latch blocks motion, relays, PWM, and lighting until disabled." => {
            "قفل برنامه‌ریزی تا زمان غیرفعال‌شدن، حرکت، رله‌ها، PWM و روشنایی را مسدود می‌کند."
        }
        "Save to board" => "ذخیره در برد",
        "Revert" => "بازگردانی",
        "Live settings" => "تنظیمات زنده",
        "Board name" => "نام برد",
        "Change icon" => "تغییر نماد",
        "Rename" => "تغییر نام",
        "Stored board name" => "نام ذخیره‌شدهٔ برد",
        "Product identity" => "هویت محصول",
        "Name storage" => "محل ذخیرهٔ نام",
        "EEPROM (persisted)" => "EEPROM (ذخیره‌شده)",
        "Not persisted" => "ذخیره نشده",
        "Not assigned" => "تعیین نشده",
        "Device path" => "مسیر دستگاه",
        "Device name" => "نام دستگاه",
        "Friendly name" => "نام خوانا",
        "Product" => "محصول",
        "Manufacturer" => "سازنده",
        "USB identity" => "شناسهٔ USB",
        "Serial number" => "شمارهٔ سریال",
        "Device instance" => "نمونهٔ دستگاه",
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
        "PWM outputs" => "خروجی‌های PWM",
        "RF transmit" => "ارسال رادیویی",
        "Addressable strip" => "نوار نور آدرس‌پذیر",
        "Addressable LED strip" => "نوار LED آدرس‌پذیر",
        "Pixel count" => "تعداد پیکسل‌ها",
        "pixels" => "پیکسل",
        "Configure" => "پیکربندی",
        "Solid color" => "رنگ یکدست",
        "Single pixel" => "یک پیکسل",
        "Color frame" => "فریم رنگی",
        "Rainbow" => "رنگین‌کمان",
        "Color" => "رنگ",
        "Fill strip" => "پر کردن نوار",
        "Pixel" => "پیکسل",
        "Apply pixel" => "اعمال پیکسل",
        "Start color" => "رنگ آغاز",
        "End color" => "رنگ پایان",
        "Send color frame" => "ارسال فریم رنگی",
        "Frame rate" => "نرخ فریم",
        "Start rainbow" => "شروع رنگین‌کمان",
        "Select effect" => "انتخاب جلوه",
        "Start effect" => "شروع جلوه",
        "Clear strip" => "پاک کردن نوار",
        "Stop stream" => "توقف جریان",
        "Streaming" => "در حال پخش",
        "Idle" => "آماده",
        "Unavailable" => "در دسترس نیست",
        "No addressable LED effects are advertised" => "هیچ جلوهٔ LED آدرس‌پذیری اعلام نشده است",
        "Seven-segment display" => "نمایشگر هفت‌بخشی",
        "LCD display" => "نمایشگر LCD",
        "Board macros" => "ماکروهای برد",
        "Timed effects" => "جلوه‌های زمان‌بندی‌شده",
        "Capability bits" => "بیت‌های قابلیت",
        "Controls" => "کنترل‌ها",
        "Relays" => "رله‌ها",
        "PWM channels" => "کانال‌های PWM",
        "Peripherals" => "تجهیزات جانبی",
        "Macros" => "ماکروها",
        "Lighting effects" => "جلوه‌های نورپردازی",
        "Live front-panel state" => "وضعیت زندهٔ پنل جلویی",
        "Live physical display and board controls" => "نمایشگر فیزیکی زنده و کنترل‌های برد",
        "Live physical display" => "نمایشگر فیزیکی زنده",
        "Physical board keys" => "کلیدهای فیزیکی برد",
        "Read the exact state from the physical board" => "خواندن وضعیت دقیق از برد فیزیکی",
        "Reading the physical front panel" => "در حال خواندن پنل جلویی فیزیکی",
        "Pealayer is requesting the exact display frame from PCController." => {
            "Pealayer در حال دریافت فریم دقیق نمایشگر از PCController است."
        }
        "Physical front panel refreshed" => "پنل جلویی فیزیکی به‌روز شد",
        "Technical readback" => "خوانش فنی",
        "Display active" => "نمایشگر فعال",
        "Display idle" => "نمایشگر غیرفعال",
        "Schema" => "طرحواره",
        "Page" => "صفحه",
        "Previous" => "قبلی",
        "Next" => "بعدی",
        "Decrease" => "کاهش",
        "Increase" => "افزایش",
        "Select" => "انتخاب",
        "No exact seven-segment frame is available." => "فریم دقیق نمایشگر هفت‌بخشی در دسترس نیست.",
        "Send the same front-panel key press through PCController" => {
            "ارسال همان فشار کلید پنل جلویی از طریق PCController"
        }
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
        "Playback speed" => "سرعت پخش",
        "Hold-to-fast-forward speed" => "سرعت جلورفتن با نگه‌داشتن",
        "Normal speed used for playback and restored after a temporary fast-forward" => {
            "سرعت عادی پخش که پس از جلورفتن موقت بازیابی می‌شود"
        }
        "Speed used while holding the configured temporary fast-forward gesture" => {
            "سرعت هنگام نگه‌داشتن حرکت جلورفتن موقت"
        }
        "Connection" => "اتصال",
        "Reconnect" => "اتصال دوباره",
        "Refresh live status" => "تازه‌سازی وضعیت زنده",
        "View options" => "گزینه‌های نمایش",
        "Compact controls" => "کنترل‌های فشرده",
        "Show raw relays" => "نمایش رله‌های خام",
        "Prefix relay identifiers" => "نمایش شناسهٔ رله پیش از نام",
        "Connection behavior" => "رفتار اتصال",
        "Connect automatically" => "اتصال خودکار",
        "Pause playback on disconnect" => "توقف پخش هنگام قطع اتصال",
        "Hardware endpoint" => "نشانی سخت‌افزار",
        "Connected" => "متصل",
        "Motion controls" => "کنترل‌های حرکتی",
        "Output controls" => "کنترل خروجی‌ها",
        "Update PWM outputs while dragging" => "به‌روزرسانی زندهٔ خروجی‌های PWM هنگام کشیدن",
        "Activate output buttons when pressed" => "فعال‌سازی دکمه‌های خروجی هنگام فشردن",
        "Motion / seat controls" => "کنترل حرکت / صندلی",
        "Moving up color" => "رنگ حرکت رو به بالا",
        "Moving down color" => "رنگ حرکت رو به پایین",
        "Button behavior" => "رفتار دکمه",
        "Hold" => "نگه‌داشتن",
        "Push" => "فشاری",
        "Toggle" => "تغییر حالت",
        "Move only while the button is held" => "حرکت فقط هنگام نگه‌داشتن دکمه",
        "Keep moving until Stop is pressed" => "ادامهٔ حرکت تا فشردن توقف",
        "Show raw relay controls" => "نمایش کنترل‌های رلهٔ خام",
        "Show raw relay controls below seat controls" => "نمایش کنترل‌های رلهٔ خام زیر کنترل صندلی",
        "Prefix relay captions with channel identifiers" => "نمایش شناسهٔ کانال پیش از نام رله",
        "Raw relays" => "رله‌های خام",
        "Raw relay" => "رلهٔ خام",
        "Relay" => "رله",
        "Board controls" => "کنترل‌های برد",
        "Board tools" => "ابزارهای برد",
        "Message for the board displays" => "پیام برای نمایشگرهای برد",
        "Segments" => "نمایشگر هفت‌بخشی",
        "Both displays" => "هر دو نمایشگر",
        "Bits" => "بیت‌ها",
        "Protocol" => "پروتکل",
        "Hex or decimal" => "هگز یا دهدهی",
        "Ready" => "آماده",
        "Invalid code" => "کد نامعتبر",
        "Transmit" => "ارسال",
        "State not sampled by the board" => "وضعیت توسط برد نمونه‌برداری نشده است",
        "Toggle on press" => "تغییر حالت با فشردن",
        "Run only while held" => "حرکت فقط هنگام نگه‌داشتن",
        "Use one-row compact hardware controls" => "استفاده از کنترل سخت‌افزار فشردهٔ یک‌ردیفه",
        "Use compact timeline track rows" => "استفاده از ردیف‌های فشرده در خط زمان",
        "Compact track rows" => "ردیف‌های فشردهٔ خط زمان",
        "Toggle mode keeps a direction active until another action is chosen. Hold mode sends Stop when the pressed direction is released." => {
            "در حالت تغییر وضعیت، جهت تا انتخاب فرمان بعدی فعال می‌ماند. در حالت نگه‌داشتن، با رهاکردن دکمه فرمان توقف فرستاده می‌شود."
        }
        "Video surface" => "سطح ویدئو",
        "Status bar" => "نوار وضعیت",
        "Hide" => "پنهان‌کردن",
        "Hide channel" => "پنهان‌کردن کانال",
        "Hide this channel from the Hardware Monitor" => "پنهان‌کردن این کانال از پایشگر سخت‌افزار",
        "Hidden channels" => "کانال‌های پنهان",
        "Show channel" => "نمایش کانال",
        "All channels are hidden" => "همهٔ کانال‌ها پنهان هستند",
        "hidden" => "پنهان",
        "Lock channel" => "قفل‌کردن کانال",
        "Channel is locked in PCController" => "کانال در PCController قفل است",
        "Prevent control changes until this channel is unlocked" => {
            "جلوگیری از تغییر کنترل تا زمانی که قفل کانال باز شود"
        }
        "Lock" => "قفل",
        "Prevent live control" => "جلوگیری از کنترل زنده",
        "Visibility" => "نمایانی",
        "Show in Hardware Monitor" => "نمایش در پایشگر سخت‌افزار",
        "Drag to reorder channel" => "برای تغییر ترتیب کانال بکشید",
        "Saving channel order..." => "در حال ذخیرهٔ ترتیب کانال‌ها…",
        "Channel order saved" => "ترتیب کانال‌ها ذخیره شد",
        "Unable to reorder these channels" => "تغییر ترتیب این کانال‌ها ممکن نیست",
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
        "No effects" => "جلوه‌ای موجود نیست",
        "Until stopped" => "تا زمان توقف",
        "Loading hardware…" => "در حال بارگذاری سخت‌افزار…",
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
        "Record effect" => "ضبط جلوه",
        "Seat motion take" => "برداشت حرکت صندلی",
        "Start board recording" => "آغاز ضبط از برد",
        "Start recording" => "آغاز ضبط",
        "Record from the current media time" => "ضبط از زمان کنونی رسانه",
        "All applied hardware actions" => "همهٔ فرمان‌های اعمال‌شدهٔ سخت‌افزار",
        "Relay snapshots in board RAM" => "نمونه‌های رله در حافظهٔ برد",
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
        "Connection problem" => "مشکل اتصال",
        "Playback problem" => "مشکل پخش",
        "Operation failed" => "عملیات ناموفق بود",
        "Pealayer could not complete the requested operation." => {
            "پی‌لیر نتوانست عملیات درخواستی را کامل کند."
        }
        "Technical details" => "جزئیات فنی",
        "Copy details" => "رونوشت جزئیات",
        "Close" => "بستن",
        "Save" => "ذخیره",
        "Preferences" => "تنظیمات",
        "Preferences saved" => "تنظیمات ذخیره شد",
        "Unsaved changes" => "تغییرات ذخیره‌نشده",
        "Unsaved preferences" => "تنظیمات ذخیره‌نشده",
        "Save your changes before closing Preferences?" => {
            "پیش از بستن تنظیمات، تغییرات ذخیره شوند؟"
        }
        "Preferences reloaded from disk" => "تنظیمات از دیسک بازخوانی شد",
        "No changes to save" => "تغییری برای ذخیره وجود ندارد",
        "Config file" => "پروندهٔ پیکربندی",
        "Reload from disk" => "بازخوانی از دیسک",
        "Edit config file" => "ویرایش پروندهٔ پیکربندی",
        "Edit config file in an external editor" => "ویرایش پروندهٔ پیکربندی در ویرایشگر بیرونی",
        "Open containing folder" => "باز کردن پوشهٔ پرونده",
        "Copy full path" => "رونوشت مسیر کامل",
        "Configuration path copied" => "مسیر کامل پیکربندی رونویسی شد",
        "Import configuration" => "درون‌ریزی پیکربندی",
        "Export configuration" => "برون‌ریزی پیکربندی",
        "Configuration imported; review and save" => {
            "پیکربندی درون‌ریزی شد؛ آن را بررسی و ذخیره کنید"
        }
        "Configuration exported" => "پیکربندی برون‌ریزی شد",
        "Automatically reload configuration changes" => "بازخوانی خودکار تغییرات پیکربندی",
        "Opened configuration in the external editor" => {
            "پروندهٔ پیکربندی در ویرایشگر بیرونی باز شد"
        }
        "Opened the configuration folder" => "پوشهٔ پیکربندی باز شد",
        "Configuration changed on disk; reload it or save your draft" => {
            "پیکربندی روی دیسک تغییر کرده است؛ آن را بازخوانی یا پیش‌نویس را ذخیره کنید"
        }
        "Open Location / URL" => "باز کردن نشانی یا پیوند",
        "Playback history" => "تاریخچهٔ پخش",
        "Remember the last position of local and remote media" => {
            "به‌خاطر سپردن آخرین موقعیت رسانه‌های محلی و راه‌دور"
        }
        "Maximum remembered videos" => "بیشترین تعداد ویدئوهای به‌خاطر سپرده‌شده",
        "Clear remembered positions" => "پاک‌کردن موقعیت‌های به‌خاطر سپرده‌شده",
        "Resumed at" => "ادامه از",
        "Enter a remote media location" => "نشانی رسانهٔ راه‌دور را وارد کنید",
        "Enter a media URL (HTTP/HTTPS, HLS, RTSP, RTMP, SRT, UDP, or TCP):" => {
            "نشانی رسانه (HTTP/HTTPS،‏ HLS،‏ RTSP،‏ RTMP،‏ SRT،‏ UDP یا TCP) را وارد کنید:"
        }
        "Enter direct video URL, HTTP/HTTPS stream, or HLS link:" => {
            "نشانی مستقیم ویدئو، جریان HTTP/HTTPS یا پیوند HLS را وارد کنید:"
        }
        "Wrap long URLs in a text area" => "پیوندهای بلند در کادر متنی چندخطی شکسته شوند",
        "Supported: HTTP/HTTPS, HLS, RTSP, RTMP, SRT, UDP, TCP, and RIST." => {
            "پشتیبانی‌شده: HTTP/HTTPS،‏ HLS،‏ RTSP،‏ RTMP،‏ SRT،‏ UDP،‏ TCP و RIST."
        }
        "Use proxy" => "استفاده از پراکسی",
        "Configure proxy" => "پیکربندی پراکسی",
        "Recent locations" => "نشانی‌های اخیر",
        "Clear all" => "پاک‌کردن همه",
        "No recent remote locations." => "نشانی راه‌دور اخیری وجود ندارد.",
        "Remove from history" => "حذف از تاریخچه",
        "Expand recent URL history by default" => "تاریخچهٔ نشانی‌های اخیر به‌طور پیش‌فرض باز باشد",
        "Fetch remote media information automatically" => {
            "اطلاعات رسانهٔ راه‌دور به‌طور خودکار دریافت شود"
        }
        "Fetch a remote media thumbnail automatically" => {
            "تصویر بندانگشتی رسانهٔ راه‌دور به‌طور خودکار دریافت شود"
        }
        "Use a proxy for URL inspection" => "استفاده از پراکسی برای بررسی نشانی",
        "Use a proxy for remote inspection and playback" => {
            "استفاده از پراکسی برای بررسی و پخش رسانهٔ راه‌دور"
        }
        "Inherited proxy" => "پراکسی به‌ارث‌رسیده",
        "Custom proxy URL" => "نشانی پراکسی سفارشی",
        "Enter a complete HTTP or HTTPS proxy URL." => {
            "یک نشانی کامل HTTP یا HTTPS برای پراکسی وارد کنید."
        }
        "Leave the custom proxy blank to inherit the operating-system environment." => {
            "برای استفاده از تنظیمات محیط سیستم‌عامل، نشانی پراکسی سفارشی را خالی بگذارید."
        }
        "MPV playback proxy" => "پراکسی پخش MPV",
        "MPV requires an http:// proxy URL; URL inspection can still use HTTPS proxy URLs." => {
            "MPV به نشانی پراکسی http:// نیاز دارد؛ بررسی نشانی همچنان می‌تواند از پراکسی HTTPS استفاده کند."
        }
        "MPV applies this proxy to supported HTTP media requests. HTTPS and extractor proxy support depends on the bundled MPV and FFmpeg backends." => {
            "MPV این پراکسی را برای درخواست‌های HTTP پشتیبانی‌شده به‌کار می‌برد. پشتیبانی پراکسی HTTPS و استخراج‌کننده به نسخه‌های همراه MPV و FFmpeg بستگی دارد."
        }
        "File associations" => "وابستگی‌های پرونده",
        "Registered media types" => "گونه‌های رسانهٔ ثبت‌شده",
        "Register Pealayer with the operating system, then choose it as the default app for the media types you want." => {
            "Pealayer را در سیستم‌عامل ثبت کنید، سپس آن را برای گونه‌های رسانهٔ دلخواه به‌عنوان برنامهٔ پیش‌فرض برگزینید."
        }
        "Register as a media player" => "ثبت به‌عنوان پخش‌کنندهٔ رسانه",
        "Remove file associations" => "حذف وابستگی‌های پرونده",
        "Removed media types" => "گونه‌های رسانهٔ حذف‌شده",
        "Clear remote history" => "پاک‌کردن تاریخچهٔ راه‌دور",
        "Clear from history" => "پاک‌کردن از تاریخچه",
        "Play when a recent location is clicked" => "با کلیک روی نشانی اخیر، آن را پخش کن",
        "Inspect now" => "بررسی اکنون",
        "Fetching details…" => "در حال دریافت جزئیات…",
        "Refresh details" => "تازه‌سازی جزئیات",
        "Retry details" => "تلاش دوباره برای جزئیات",
        "Fetch details" => "دریافت جزئیات",
        "Fetching details" => "در حال دریافت جزئیات",
        "Preparing remote media details…" => "در حال آماده‌سازی جزئیات رسانهٔ راه‌دور…",
        "Automatic remote information is disabled." => {
            "دریافت خودکار اطلاعات رسانهٔ راه‌دور غیرفعال است."
        }
        "Invalid media location" => "نشانی رسانه نامعتبر است",
        "Problem" => "مشکل",
        "Entered value" => "مقدار واردشده",
        "How to fix" => "راه‌حل",
        "Enter a complete supported URL including its protocol." => {
            "یک نشانی کامل و پشتیبانی‌شده همراه با پروتکل آن وارد کنید."
        }
        "Could not fetch remote media details" => "جزئیات رسانهٔ راه‌دور دریافت نشد",
        "Target" => "مقصد",
        "Reason" => "دلیل",
        "System default" => "پیش‌فرض سیستم",
        "Disabled" => "غیرفعال",
        "Proxy" => "پراکسی",
        "Next step" => "گام بعدی",
        "Check the address or connection, then retry the details request." => {
            "نشانی یا اتصال را بررسی کنید و سپس دریافت جزئیات را دوباره امتحان کنید."
        }
        "Cut" => "برش",
        "Copy" => "رونوشت",
        "Select All" => "انتخاب همه",
        "Valid media location" => "نشانی معتبر رسانه",
        "Remote media information" => "اطلاعات رسانهٔ راه‌دور",
        "Enter a location to validate and inspect it." => {
            "برای اعتبارسنجی و بررسی، یک نشانی وارد کنید."
        }
        "Waiting for a valid HTTP or HTTPS URL…" => "در انتظار یک نشانی معتبر HTTP یا HTTPS…",
        "Checking" => "در حال بررسی",
        "Status" => "وضعیت",
        "Final URL" => "نشانی نهایی",
        "File name" => "نام پرونده",
        "Content type" => "نوع محتوا",
        "Remote size" => "اندازهٔ راه‌دور",
        "Byte ranges" => "بازه‌های بایتی",
        "Last modified" => "آخرین تغییر",
        "Server" => "سرور",
        "Response time" => "زمان پاسخ",
        "Thumbnail at 20%" => "تصویر بندانگشتی در ۲۰٪",
        "Initial frame" => "نخستین قاب",
        "Thumbnail unavailable" => "تصویر بندانگشتی در دسترس نیست",
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
        "Accent color" => "رنگ تأکیدی",
        "System accent" => "رنگ تأکیدی سامانه",
        "Pealayer green" => "سبز پی‌لیر",
        "Windows blue" => "آبی ویندوز",
        "macOS blue" => "آبی مک‌اواس",
        "Custom" => "سفارشی",
        "Custom accent" => "رنگ تأکیدی سفارشی",
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
        "Selected cue" => "نشانهٔ انتخاب‌شده",
        "1 cue" => "۱ نشانه",
        "cues" => "نشانه",
        "selected" => "انتخاب‌شده",
        "controller-owned" => "تحت مدیریت کنترل‌گر",
        "Multiple cues selected" => "چند نشانه انتخاب شده است",
        "Changes apply to every compatible cue" => "تغییرات روی همهٔ نشانه‌های سازگار اعمال می‌شود",
        "Timing" => "زمان‌بندی",
        "Exact timeline placement and length" => "جای‌گذاری و مدت دقیق در خط زمانی",
        "Starts" => "آغاز",
        "Go to cue" => "رفتن به نشانه",
        "Untitled effect" => "جلوهٔ بی‌نام",
        "No cue selected" => "هیچ نشانه‌ای انتخاب نشده است",
        "Select a cue on the timeline to manage it" => {
            "برای مدیریت، نشانه‌ای را در خط زمانی انتخاب کنید"
        }
        "Cue unavailable" => "نشانه در دسترس نیست",
        "The selected cue is no longer on the timeline" => "نشانهٔ انتخاب‌شده دیگر در خط زمانی نیست",
        "Source" => "منبع",
        "Addressable lighting" => "نورپردازی آدرس‌پذیر",
        "Hardware macro" => "ماکروی سخت‌افزاری",
        "PCController effect" => "جلوهٔ PCController",
        "Relay sequence" => "توالی رله",
        "Timeline effect" => "جلوهٔ خط زمانی",
        "No live hardware outputs" => "هیچ خروجی سخت‌افزاری زنده‌ای در دسترس نیست",
        "Output" => "خروجی",
        "Set hardware target" => "تعیین مقصد سخت‌افزاری",
        "Target mismatch" => "ناهماهنگی مقصد",
        "Configured output" => "خروجی پیکربندی‌شده",
        "Move cue to matching track" => "انتقال نشانه به ترک سازگار",
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
        "LIVE" => "زنده",
        "Determining duration…" => "در حال تعیین مدت…",
        "Open media to see its duration." => "برای دیدن مدت، رسانه‌ای باز کنید.",
        "MPV is still reading media metadata. Duration and seeking will update when available." => {
            "MPV هنوز در حال خواندن فرادادهٔ رسانه است. مدت و امکان جابه‌جایی پس از آماده‌شدن به‌روز می‌شوند."
        }
        "This live or duration-less source has no fixed endpoint or seek range." => {
            "این منبع زنده یا بدون مدت، نقطهٔ پایان یا بازهٔ جابه‌جایی ثابتی ندارد."
        }
        "Showing time remaining. Click to show total duration." => {
            "زمان باقی‌مانده نمایش داده می‌شود. برای نمایش مدت کل کلیک کنید."
        }
        "Showing total duration. Click to show time remaining." => {
            "مدت کل نمایش داده می‌شود. برای نمایش زمان باقی‌مانده کلیک کنید."
        }
        "Open media to seek." => "برای جابه‌جایی، رسانه‌ای باز کنید.",
        "Duration is still being determined; seeking will become available when MPV reports a timeline." => {
            "مدت هنوز در حال تعیین است؛ وقتی MPV خط زمانی را گزارش کند جابه‌جایی فعال می‌شود."
        }
        "This live or duration-less source has no fixed seek range." => {
            "این منبع زنده یا بدون مدت، بازهٔ جابه‌جایی ثابتی ندارد."
        }
        "This media reports a duration but does not support seeking." => {
            "این رسانه مدت را گزارش می‌کند اما از جابه‌جایی پشتیبانی نمی‌کند."
        }
        "Seek through the media timeline." => "در خط زمانی رسانه جابه‌جا شوید.",
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
        "EMERGENCY STOP ACTIVE - ALL OUTPUTS DISABLED" => {
            "توقف اضطراری فعال است — همهٔ خروجی‌ها غیرفعال‌اند"
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
    if language != AppLanguage::Persian && !contains_arabic_script(logical) {
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

/// Whether `text` contains Arabic-script Unicode, including Persian-specific
/// letters and the presentation forms used by the static-label compatibility
/// renderer.
pub fn contains_arabic_script(text: &str) -> bool {
    text.chars().any(|character| {
        matches!(
            character,
            '\u{0600}'..='\u{06FF}'
                | '\u{0750}'..='\u{077F}'
                | '\u{08A0}'..='\u{08FF}'
                | '\u{FB50}'..='\u{FDFF}'
                | '\u{FE70}'..='\u{FEFF}'
        )
    })
}

/// Keep editable Persian/Arabic content on its natural edge. Since egui 0.35,
/// HarfRust performs contextual shaping for `TextEdit`; this alignment helper
/// supplies the missing field-level RTL placement without changing the stored
/// logical Unicode or corrupting cursor/edit operations.
pub fn input_alignment(rtl_ui: bool, logical: &str) -> egui::Align {
    if rtl_ui || contains_arabic_script(logical) {
        egui::Align::Max
    } else {
        egui::Align::Min
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

    #[test]
    fn persian_input_alignment_follows_content_even_in_ltr_ui() {
        assert_eq!(input_alignment(false, "Cinema relay"), egui::Align::Min);
        assert_eq!(input_alignment(false, "رله صندلی"), egui::Align::Max);
        assert_eq!(input_alignment(true, "Cinema relay"), egui::Align::Max);
    }

    #[test]
    fn bundled_vazirmatn_uses_contextual_arabic_shaping() {
        let context = egui::Context::default();
        configure_ui_fonts(&context, true);
        let mut contextual = Vec::new();
        let mut isolated = None;
        let mut output = context.run_ui(Default::default(), |ui| {
            contextual = ui
                .painter()
                .layout_no_wrap(
                    "بب".to_owned(),
                    egui::FontId::proportional(16.0),
                    egui::Color32::WHITE,
                )
                .rows[0]
                .glyphs
                .iter()
                .map(|glyph| glyph.uv_rect)
                .collect();
            isolated = Some(
                ui.painter()
                    .layout_no_wrap(
                        "ب".to_owned(),
                        egui::FontId::proportional(16.0),
                        egui::Color32::WHITE,
                    )
                    .rows[0]
                    .glyphs[0]
                    .uv_rect,
            );
        });
        output.textures_delta.clear();
        assert!(contextual.len() >= 2);
        assert!(
            contextual.iter().any(|glyph| Some(*glyph) != isolated),
            "Arabic letters must use contextual joined forms rather than isolated glyphs"
        );
    }
}
