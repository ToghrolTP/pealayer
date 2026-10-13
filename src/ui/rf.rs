use crate::app::PealayerApp;
use crate::ui::dropdown::DropdownUiExt;
use eframe::egui;
use serde_json::{Value, json};
use std::time::Duration;

pub const OPERATIONS: &[&str] = &[
    "catalog",
    "learn.start",
    "learn.status",
    "learn.cancel",
    "map",
    "remove",
    "clear",
    "transmit",
    "binding.put",
    "binding.remove",
];

pub struct RfState {
    pub open: bool,
    pub pending: bool,
    pub error: String,
    pub error_detail: Value,
    pub catalog: Value,
    pub last_result: Value,
    pub last_operation: String,
    error_is_catalog: bool,
    pending_name: String,
    tab: usize,
    draft: Value,
    previous_name: String,
    allow_keyboard: bool,
    code: String,
    bits: u8,
    protocol: u8,
    pulse_us: u16,
    repeats: u8,
    board_map: Value,
    manual_signal: bool,
    assignment_control: Option<String>,
}

fn new_binding() -> Value {
    json!({"name":"", "enabled":true,"cooldown_ms":250,
        "match":{"kind":"rf.gesture","source":"rf","rf_code":0,"rf_bits":24,"rf_protocol":1,"gesture":"down"},
        "actions":[{"type":"control","action_id":""}]})
}

impl Default for RfState {
    fn default() -> Self {
        Self {
            open: false,
            pending: false,
            error: String::new(),
            error_detail: Value::Null,
            catalog: Value::Null,
            last_result: Value::Null,
            last_operation: String::new(),
            error_is_catalog: false,
            pending_name: String::new(),
            tab: 0,
            draft: new_binding(),
            previous_name: String::new(),
            allow_keyboard: false,
            code: String::new(),
            bits: 24,
            protocol: 1,
            pulse_us: 0,
            repeats: 1,
            board_map: json!({"id":null,"action":""}),
            manual_signal: false,
            assignment_control: None,
        }
    }
}

impl RfState {
    fn begin_control_assignment(&mut self, key: &str, name: &str) {
        self.open = true;
        self.tab = 0;
        self.draft = new_binding();
        self.draft["name"] = json!(name);
        self.previous_name.clear();
        self.assignment_control = Some(key.to_owned());
    }
    pub(crate) fn receive(&mut self, event: Value) {
        if event["kind"] != "rf.receive" || event["source"] != "board" {
            return;
        }
        let Some(time) = event["time"]
            .as_str()
            .and_then(|time| time.parse::<jiff::Timestamp>().ok())
        else {
            return;
        };
        if array(&self.catalog["activity"]).iter().any(|old| {
            old["kind"] == "rf.receive"
                && ["rf_code", "rf_bits", "rf_protocol"]
                    .iter()
                    .all(|key| old[*key] == event[*key])
                && old["time"]
                    .as_str()
                    .and_then(|value| value.parse::<jiff::Timestamp>().ok())
                    .is_some_and(|old| old > time)
        }) {
            return;
        }
        if let Some(entries) = self.catalog["entries"].as_array_mut() {
            for entry in entries {
                if entry["code"] == event["rf_code"]
                    && entry["bits"] == event["rf_bits"]
                    && entry["protocol"] == event["rf_protocol"]
                    && entry["last_seen"]
                        .as_str()
                        .and_then(|value| value.parse::<jiff::Timestamp>().ok())
                        .is_none_or(|old| time >= old)
                {
                    entry["last_seen"] = event["time"].clone();
                }
            }
        }
        if !self.catalog["activity"].is_array() {
            self.catalog["activity"] = json!([]);
        }
        let activity = self.catalog["activity"].as_array_mut().unwrap();
        activity.retain(|old| {
            !(old["kind"] == "rf.receive"
                && ["rf_code", "rf_bits", "rf_protocol"]
                    .iter()
                    .all(|key| old[*key] == event[*key]))
        });
        activity.insert(0, event);
        activity.sort_by_key(|event| {
            std::cmp::Reverse(
                event["time"]
                    .as_str()
                    .and_then(|time| time.parse::<jiff::Timestamp>().ok()),
            )
        });
        activity.truncate(32);
    }
    pub fn snapshot(&self) -> Value {
        json!({"catalog":self.catalog,"pending":self.pending,"error":self.error,"error_detail":self.error_detail,"last_result":self.last_result,"last_operation":self.last_operation})
    }
    pub fn apply_catalog(&mut self, mut value: Value) {
        if value["connected"].as_bool() == Some(true)
            && value["entries_sampled"].as_bool() != Some(true)
        {
            value["entries"] = self.catalog["entries"].clone();
            value["entries_sampled"] = self.catalog["entries_sampled"].clone();
        }
        if value["hostname"] == self.catalog["hostname"]
            && let Some(entries) = value["entries"].as_array_mut()
        {
            for entry in entries {
                if let Some(old) = array(&self.catalog["entries"]).into_iter().find(|old| {
                    old["code"] == entry["code"]
                        && old["bits"] == entry["bits"]
                        && old["protocol"] == entry["protocol"]
                }) && old["last_seen"]
                    .as_str()
                    .and_then(|time| time.parse::<jiff::Timestamp>().ok())
                    > entry["last_seen"]
                        .as_str()
                        .and_then(|time| time.parse::<jiff::Timestamp>().ok())
                {
                    entry["last_seen"] = old["last_seen"].clone();
                }
            }
        }
        self.catalog = value;
    }
    pub fn complete(&mut self, operation: &str, result: Result<Value, String>) -> bool {
        self.pending = false;
        match result {
            Ok(value) => {
                if operation != "rf-catalog" || self.error_is_catalog {
                    self.error.clear();
                    self.error_detail = Value::Null;
                }
                if operation == "rf-catalog" {
                    let learning_finished = self.catalog["learning"]["active"] == true
                        && value["learning"]["active"] != true;
                    self.apply_catalog(value);
                    return learning_finished;
                }
                if operation == "rf-binding.put"
                    && self.draft["name"].as_str() == Some(self.pending_name.as_str())
                {
                    self.previous_name = self.pending_name.clone();
                }
                if operation == "rf-binding.remove" && self.previous_name == self.pending_name {
                    self.previous_name.clear();
                }
                self.last_operation = operation.into();
                self.last_result = value;
                true
            }
            Err(error) => {
                self.error = error;
                self.error_is_catalog = operation == "rf-catalog";
                false
            }
        }
    }
}

impl PealayerApp {
    pub(crate) fn request_rf(&mut self, operation: &str, params: Value) -> Result<(), String> {
        if !OPERATIONS.contains(&operation) {
            return Err("Unknown RF operation".into());
        }
        if self.rf.pending {
            // Desktop and web readers share one operation queue. Polls must not
            // overwrite a mutation's result or report a false failure.
            if operation == "catalog" {
                return Ok(());
            }
            return Err("An RF operation is still pending".into());
        }
        let pending_name = if operation == "binding.put" {
            params["binding"]["name"].as_str()
        } else {
            params["name"].as_str()
        }
        .unwrap_or_default()
        .to_owned();
        self.engine_handle.request_controller_call(
            &format!("rf-{operation}"),
            &format!("controller.rf.{operation}"),
            params,
        )?;
        self.rf.pending_name = pending_name;
        if operation != "catalog" {
            self.rf.error.clear();
            self.rf.error_detail = Value::Null;
            self.rf.error_is_catalog = false;
        }
        self.rf.pending = true;
        Ok(())
    }
}

fn request(app: &mut PealayerApp, operation: &str, params: Value) {
    if let Err(error) = app.request_rf(operation, params) {
        app.rf.error = error;
        app.rf.error_detail = Value::Null;
    }
}

fn text(ui: &mut egui::Ui, label: &str, value: &mut Value, field: &str) {
    field_label(ui,label);
    let mut current = value[field].as_str().unwrap_or_default().to_string();
    if ui
        .add(
            crate::ui::dialog::singleline_text_edit(ui, &mut current)
                .desired_width(field_width(ui)),
        )
        .changed()
    {
        value[field] = json!(current);
    }
    ui.end_row();
}

fn choices(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut Value,
    field: &str,
    options: &[(String, String)],
) {
    field_label(ui,label);
    let mut selected = value[field].as_str().unwrap_or_default().to_string();
    let caption = options
        .iter()
        .find(|(id, _)| *id == selected)
        .map(|(_, name)| name.as_str())
        .unwrap_or(if selected.is_empty() {
            "Choose…"
        } else {
            &selected
        });
    crate::ui::dropdown::ComboBox::from_id_salt((ui.id(), field))
        .selected_text(caption)
        .width(field_width(ui))
        .truncate()
        .show_ui(ui, |ui| {
            if options.is_empty() {
                ui.label("No available choices");
            }
            for (id, name) in options {
                ui.dropdown_value(&mut selected, id.clone(), name);
            }
        });
    value[field] = json!(selected);
    ui.end_row();
}

fn array(value: &Value) -> Vec<Value> {
    value.as_array().cloned().unwrap_or_default()
}
fn field_width(ui: &egui::Ui) -> f32 {
    (ui.max_rect().width() - 155.0).clamp(80.0, 420.0)
}

fn field_label(ui:&mut egui::Ui,label:&str) {
    let height=crate::ui::control_density::height(ui);
    ui.add_sized(egui::vec2(130.0,height),egui::Label::new(label).truncate()).on_hover_text(label);
}

fn signal_matches(binding: &Value, entry: &Value) -> bool {
    binding["match"]["rf_code"] == entry["code"]
        && binding["match"]["rf_bits"] == entry["bits"]
        && binding["match"]["rf_protocol"] == entry["protocol"]
}

fn last_received(catalog: &Value) -> Option<Value> {
    array(&catalog["activity"])
        .into_iter()
        .filter(|event| {
            event["kind"] == "rf.receive"
                && event["source"] == "board"
                && event["rf_code"].as_u64().is_some_and(|code| code > 0)
                && event["time"]
                    .as_str()
                    .is_some_and(|time| time.parse::<jiff::Timestamp>().is_ok())
        })
        .max_by_key(|event| {
            event["time"]
                .as_str()
                .and_then(|time| time.parse::<jiff::Timestamp>().ok())
        })
}

fn action_ready(action: &Value, catalog: &Value) -> bool {
    let value = |key| action[key].as_str().unwrap_or_default();
    match value("type") {
        "control" => control_options(catalog)
            .iter()
            .any(|(id, _)| id == value("action_id")),
        "app" => array(&catalog["applications"]).iter().any(|app| {
            (app["id"] == action["app_target"] || app["surface"] == action["app_target"])
                && app["values"]["app_actions"]
                    .as_str()
                    .unwrap_or_default()
                    .split(',')
                    .any(|kind| kind == value("app_kind") && !kind.is_empty())
        }),
        "effect" | "macro" => array(&catalog["effects"])
            .iter()
            .any(|effect| effect["reference"] == action["macro"]),
        "virtual-key" => array(&catalog["keyboard"]["allowed"]).contains(&action["virtual_key"]),
        "board" => !value("command").trim().is_empty(),
        "host" => !value("executable").trim().is_empty(),
        "script" => !value("script").trim().is_empty(),
        "emit" => !value("event").trim().is_empty(),
        "rf" => action["rf"]["code"].as_u64().unwrap_or(0) > 0,
        _ => false,
    }
}

fn control_options(catalog: &Value) -> Vec<(String, String)> {
    array(&catalog["peripherals"]["controls"])
        .iter()
        .flat_map(|control| {
            array(&control["actions"])
                .into_iter()
                .filter_map(|action| {
                    let id = action["id"].as_str()?;
                    let verb = action["verb"].as_str().unwrap_or_default();
                    let step = if matches!(verb, "increase" | "decrease") {
                        control["step_percent"]
                            .as_f64()
                            .map(|step| format!(" ({step}%)"))
                            .unwrap_or_default()
                    } else {
                        String::new()
                    };
                    Some((
                        id.into(),
                        format!(
                            "{} · {}{step}",
                            control["name"].as_str().unwrap_or("Control"),
                            action["label"]
                                .as_str()
                                .or(action["name"].as_str())
                                .unwrap_or(id)
                        ),
                    ))
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn action_summary(action: &Value, catalog: &Value) -> String {
    let value = |key| action[key].as_str().unwrap_or_default();
    match value("type") {
        "control" => control_options(catalog)
            .into_iter()
            .find(|(id, _)| id == value("action_id"))
            .map(|(_, label)| label)
            .unwrap_or_else(|| format!("Unavailable control · {}", value("action_id"))),
        "app" => format!(
            "{} · {} {}",
            value("app_target"),
            value("app_kind").replace("pealayer.", "").replace('.', " "),
            value("app_value")
        ),
        "macro" | "effect" => array(&catalog["effects"])
            .iter()
            .find(|effect| effect["reference"] == action["macro"])
            .and_then(|effect| effect["name"].as_str())
            .unwrap_or(value("macro"))
            .to_owned(),
        "board" => format!("Controller · {}", value("command")),
        "virtual-key" => format!("Keyboard · {}", value("virtual_key")),
        "host" => format!("Program · {}", value("executable")),
        "script" => format!("Script · {}", value("script")),
        "rf" => format!("Transmit · {}", action["rf"]["code"]),
        "emit" => format!("Event · {}", value("event")),
        _ => "Choose an action".into(),
    }
}

fn target_options(catalog: &Value, mapping: &Value) -> Vec<(String, String)> {
    let action = mapping["action"].as_str().unwrap_or_default();
    options(&mapping["targets"])
        .into_iter()
        .map(|(target, label)| {
            let key = match action {
                "side" => format!("seat.{target}"),
                "pwm" => format!("pwm.{target}"),
                "relay" => format!("relay.{target}"),
                _ => String::new(),
            };
            let name = array(&catalog["peripherals"]["controls"])
                .iter()
                .find(|control| control["key"] == key)
                .and_then(|control| control["name"].as_str())
                .map(str::to_owned)
                .unwrap_or(label);
            (target, name)
        })
        .collect()
}

fn board_map_valid(map: &Value, catalog: &Value) -> bool {
    if !array(&catalog["entries"])
        .iter()
        .any(|entry| entry["id"] == map["id"])
    {
        return false;
    }
    let Some(option) = array(&catalog["board_mapping_options"])
        .into_iter()
        .find(|option| option["action"] == map["action"])
    else {
        return false;
    };
    ["targets", "behaviors"].into_iter().all(|field| {
        let choices = array(&option[field]);
        choices.is_empty()
            || choices.contains(
                &map[if field == "targets" {
                    "target"
                } else {
                    "behavior"
                }],
            )
    })
}

fn board_assignment(entry: &Value) -> String {
    entry["board_assignment"]["label"]
        .as_str()
        .unwrap_or("Board assignment not reported")
        .into()
}

fn edit_board_map(entry: &Value) -> Value {
    let assignment = &entry["board_assignment"];
    let mut map = json!({"id":entry["id"],"action":assignment["action"].as_str().unwrap_or("")});
    for field in ["target", "behavior"] {
        if let Some(value) = assignment[field].as_str() {
            map[field] = json!(value);
        }
    }
    map
}

fn execution_label(binding: &Value) -> &'static str {
    if array(&binding["actions"])
        .iter()
        .any(|action| action["type"] == "app")
    {
        "PCController → application"
    } else {
        "PCController · Pealayer is only the editor"
    }
}

fn seen_label(entry: &Value) -> String {
    entry["last_seen"]
        .as_str()
        .and_then(|value| value.parse::<jiff::Timestamp>().ok())
        .map(|time| {
            time.to_zoned(jiff::tz::TimeZone::system())
                .strftime("%Y-%m-%d %H:%M:%S %:z")
                .to_string()
        })
        .unwrap_or_else(|| "Not seen in this controller session".into())
}

fn draw_error(ui: &mut egui::Ui, message: &str, detail: &Value) {
    if message.is_empty() {
        return;
    }
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.colored_label(ui.visuals().error_fg_color, message);
        if detail["retryable"] == true {
            ui.label("You can retry when the controller is ready.");
        }
        if detail.is_object() {
            ui.collapsing("Details", |ui| {
                for key in ["kind", "resource", "retry_after_ms"] {
                    if !detail[key].is_null() {
                        ui.label(format!("{}: {}", key.replace('_', " "), detail[key]));
                    }
                }
            });
        }
    });
}

fn learned_selector(ui: &mut egui::Ui, signal: &mut Value, catalog: &Value) {
    field_label(ui,"Learned button");
    let entries = array(&catalog["entries"]);
    let caption = entries
        .iter()
        .find(|entry| {
            signal["rf_code"] == entry["code"]
                && signal["rf_bits"] == entry["bits"]
                && signal["rf_protocol"] == entry["protocol"]
        })
        .map(|entry| {
            format!(
                "RF {} · {}",
                entry["id"],
                entry["code_display"].as_str().unwrap_or_default()
            )
        })
        .unwrap_or_else(|| "Choose a learned button…".into());
    crate::ui::dropdown::ComboBox::from_id_salt((ui.id(), "learned_button"))
        .selected_text(caption)
        .width(field_width(ui))
        .truncate()
        .show_ui(ui, |ui| {
            for entry in entries {
                let selected = signal["rf_code"] == entry["code"]
                    && signal["rf_bits"] == entry["bits"]
                    && signal["rf_protocol"] == entry["protocol"];
                if ui
                    .dropdown_choice(
                        selected,
                        format!(
                            "RF {} · {}",
                            entry["id"],
                            entry["code_display"].as_str().unwrap_or_default()
                        ),
                    )
                    .clicked()
                {
                    for (to, from) in [
                        ("rf_code", "code"),
                        ("rf_bits", "bits"),
                        ("rf_protocol", "protocol"),
                    ] {
                        signal[to] = entry[from].clone();
                    }
                }
            }
        });
    ui.end_row();
}
fn options(value: &Value) -> Vec<(String, String)> {
    array(value)
        .iter()
        .filter_map(Value::as_str)
        .map(|v| (v.into(), v.replace(['.', '-', '_'], " ")))
        .collect()
}
fn number(ui: &mut egui::Ui, label: &str, value: &mut Value, field: &str, min: u64, max: u64) {
    field_label(ui,label);
    let mut n = value[field].as_u64().unwrap_or(min);
    if ui
        .add(egui::DragValue::new(&mut n).range(min..=max))
        .changed()
    {
        value[field] = json!(n);
    }
    ui.end_row();
}

fn action_editor(ui: &mut egui::Ui, action: &mut Value, catalog: &Value) {
    egui::Grid::new((ui.id(), "action_fields"))
        .num_columns(2)
        .spacing([16.0, 6.0])
        .show(ui, |ui| {
            let previous_type = action["type"].clone();
            choices(
                ui,
                "Action",
                action,
                "type",
                &options(&catalog["action_types"]),
            );
            if previous_type != action["type"] {
                *action = json!({"type":action["type"]});
            }
            match action["type"].as_str().unwrap_or_default() {
                "app" => {
                    let applications = array(&catalog["applications"]);
                    let mut targets = Vec::new();
                    for app in &applications {
                        if let (Some(surface), Some(id)) =
                            (app["surface"].as_str(), app["id"].as_str())
                        {
                            if !targets.iter().any(|(key, _)| key == surface) {
                                targets.push((surface.into(), format!("{surface} (all)")));
                            }
                            targets.push((id.into(), id.into()));
                        }
                    }
                    let previous_target = action["app_target"].clone();
                    choices(ui, "Application", action, "app_target", &targets);
                    if previous_target != action["app_target"] {
                        action["app_kind"] = json!("");
                        action["app_value"] = json!("");
                    }
                    let target = action["app_target"].as_str().unwrap_or_default();
                    let mut kinds = Vec::new();
                    for app in applications {
                        if app["id"].as_str() == Some(target)
                            || app["surface"].as_str() == Some(target)
                        {
                            for key in app["values"]["app_actions"]
                                .as_str()
                                .unwrap_or_default()
                                .split(',')
                                .filter(|s| !s.is_empty())
                            {
                                if !kinds.iter().any(|(id, _)| id == key) {
                                    kinds.push((key.into(), key.replace('.', " ")));
                                }
                            }
                        }
                    }
                    let previous_kind = action["app_kind"].clone();
                    choices(ui, "Command", action, "app_kind", &kinds);
                    if previous_kind != action["app_kind"] {
                        action["app_value"] = json!("");
                    }
                    match action["app_kind"].as_str().unwrap_or_default() {
                        "pealayer.mute.set" | "pealayer.fullscreen.set" => choices(
                            ui,
                            "Value",
                            action,
                            "app_value",
                            &[("true".into(), "On".into()), ("false".into(), "Off".into())],
                        ),
                        "pealayer.rate.set" => choices(
                            ui,
                            "Speed",
                            action,
                            "app_value",
                            &["0.5", "0.75", "1", "1.25", "1.5", "2"]
                                .map(|value| (value.into(), format!("{value}×"))),
                        ),
                        "pealayer.volume.set"
                        | "pealayer.seek"
                        | "pealayer.seek_to"
                        | "pealayer.seek_absolute"
                        | "pealayer.chapter.set"
                        | "pealayer.open"
                        | "pealayer.command"
                        | "pealayer.workspace.set" => text(ui, "Value", action, "app_value"),
                        "pealayer.play"
                        | "pealayer.pause"
                        | "pealayer.toggle"
                        | "pealayer.stop"
                        | "pealayer.next"
                        | "pealayer.previous"
                        | "pealayer.chapter.next"
                        | "pealayer.chapter.previous"
                        | "pealayer.mute.toggle"
                        | "pealayer.fullscreen.toggle"
                        | "pealayer.window.activate"
                        | "pealayer.window.minimize"
                        | "pealayer.window.maximize"
                        | "pealayer.window.restore"
                        | "pealayer.quit" => {}
                        _ => text(ui, "Value (advanced)", action, "app_value"),
                    }
                }
                "control" => {
                    choices(
                        ui,
                        "Control",
                        action,
                        "action_id",
                        &control_options(catalog),
                    );
                }
                "effect" | "macro" => {
                    let effects = array(&catalog["effects"])
                        .iter()
                        .filter_map(|e| {
                            e["name"].as_str().map(|name| {
                                (e["reference"].as_str().unwrap_or(name).into(), name.into())
                            })
                        })
                        .collect::<Vec<_>>();
                    choices(ui, "Effect", action, "macro", &effects);
                }
                "board" => text(ui, "Controller command", action, "command"),
                "virtual-key" => {
                    choices(
                        ui,
                        "Controller host key",
                        action,
                        "virtual_key",
                        &options(&catalog["keyboard"]["allowed"]),
                    );
                    number(ui, "Hold (ms)", action, "hold_ms", 10, 1000);
                }
                "host" | "script" => {
                    let field = if action["type"] == "host" {
                        "executable"
                    } else {
                        "script"
                    };
                    text(ui, "Program", action, field);
                    ui.label("Arguments (one per line)");
                    let mut args = array(&action["args"])
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join("\n");
                    if ui
                        .add(egui::TextEdit::multiline(&mut args).desired_rows(2))
                        .changed()
                    {
                        action["args"] = json!(args.lines().collect::<Vec<_>>());
                    }
                    ui.end_row();
                    ui.label("Launch");
                    let mut detached = action["detached"].as_bool().unwrap_or(false);
                    if ui.checkbox(&mut detached, "Launch independently").changed() {
                        action["detached"] = json!(detached);
                    }
                    ui.end_row();
                }
                "emit" => text(ui, "Event", action, "event"),
                "rf" => {
                    if !action["rf"].is_object() {
                        action["rf"] =
                            json!({"code":0,"bits":24,"protocol":1,"pulse_us":0,"repeats":1});
                    }
                    for (label, field, min, max) in [
                        ("Code", "code", 1, u32::MAX as u64),
                        ("Bits", "bits", 1, 32),
                        ("Protocol", "protocol", 1, 12),
                        ("Pulse (µs)", "pulse_us", 0, 65535),
                        ("Repeats", "repeats", 1, 20),
                    ] {
                        number(ui, label, &mut action["rf"], field, min, max);
                    }
                }
                _ => {}
            }
        });
}

pub(crate) fn open_control_assignment(app: &mut PealayerApp, control: &crate::four_d::controller::HardwareControl) {
    app.rf.begin_control_assignment(&control.key, &control.name);
    if let Err(error) = app.request_rf("catalog", json!({"read_board": true})) { app.set_osd(error); }
}

fn scoped_action_catalog(catalog: &Value, key: Option<&str>) -> Value {
    let mut scoped = catalog.clone();
    if let Some(key) = key {
        scoped["peripherals"]["controls"] = json!(array(&catalog["peripherals"]["controls"])
            .into_iter().filter(|control| control["key"] == key).collect::<Vec<_>>());
        scoped["action_types"] = json!(["control"]);
    }
    scoped
}

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if !app.rf.open {
        return;
    }
    let live_connected = app
        .engine_handle
        .is_connected
        .load(std::sync::atomic::Ordering::Acquire);
    if app.rf.pending || app.rf.catalog["learning"]["active"] == true {
        ui.ctx().request_repaint_after(Duration::from_millis(250));
    }
    let geometry = crate::ui::dialog::bounded_geometry(
        ui.ctx().content_rect(),
        24.0,
        egui::vec2(820.0, 610.0),
        egui::vec2(480.0, 380.0),
        egui::vec2(980.0, 760.0),
    );
    let mut open = true;
    let catalog = app.rf.catalog.clone();
    egui::Window::new(format!("{} RF controls",crate::ui::icons::RADIO)).id(egui::Id::new("rf_manager"))
        .title_bar(false).default_rect(geometry.default_rect).min_size(geometry.min_size).max_size(geometry.max_size)
        .constrain_to(geometry.bounds).resizable(true).collapsible(false).order(egui::Order::Foreground)
        .frame(crate::ui::dialog::opaque_window_frame(ui)).show(ui.ctx(),|ui| {
            // Keep hover paint inside allocated bounds, including closed selectors.
            ui.style_mut().interaction.selectable_labels=false;
            ui.visuals_mut().widgets.inactive.expansion=0.0;
            ui.visuals_mut().widgets.hovered.expansion=0.0;
            ui.visuals_mut().widgets.active.expansion=0.0;
            ui.visuals_mut().widgets.open.expansion=0.0;
            ui.horizontal(|ui| {
                ui.strong(format!("{} RF controls",crate::ui::icons::RADIO));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui| {
                    if crate::ui::dialog::close_button(ui,"Close RF controls").clicked() {open=false;}
                });
            });
            ui.horizontal(|ui| {
                for (i,caption) in ["Assignments","Remotes","Transmit","Activity"].iter().enumerate() { ui.dropdown_value(&mut app.rf.tab,i,*caption); }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui| {
                    if ui.add_enabled(!app.rf.pending,egui::Button::new(crate::ui::icons::ARROW_CLOCKWISE)).on_hover_text("Refresh learned buttons").clicked() { request(app,"catalog",json!({"read_board":true})); }
                    if app.rf.pending { ui.spinner(); }
                });
            });
            ui.separator();
            draw_error(ui,&app.rf.error,&app.rf.error_detail);
            if let Some(error)=catalog["board_error"].as_str() { ui.colored_label(ui.visuals().warn_fg_color,error); }
            egui::ScrollArea::vertical().id_salt("rf_body").auto_shrink([false,false]).show(ui,|ui| {
                ui.set_min_width(ui.available_width());
                match app.rf.tab {
                    0=> {
                        ui.horizontal(|ui| {
                            if ui.button(format!("{} New assignment",crate::ui::icons::PLUS)).clicked() { app.rf.draft=new_binding(); app.rf.previous_name.clear(); }
                            if let Some(event)=last_received(&catalog) {
                                if ui.button("Use last received").clicked() {
                                    for (to,from) in [("rf_code","rf_code"),("rf_bits","rf_bits"),("rf_protocol","rf_protocol")] { app.rf.draft["match"][to]=event[from].clone(); }
                                }
                            }
                        });
                        for binding in array(&catalog["bindings"]) {
                            ui.push_id(binding["name"].as_str().unwrap_or_default(),|ui| { egui::Frame::group(ui.style()).show(ui,|ui| { ui.horizontal(|ui| {
                                let mut enabled=binding["enabled"].as_bool().unwrap_or(false);
                                if ui.checkbox(&mut enabled,"").changed() { let mut updated=binding.clone(); updated["enabled"]=json!(enabled); request(app,"binding.put",json!({"binding":updated})); }
                                if ui.dropdown_choice(app.rf.previous_name==binding["name"].as_str().unwrap_or_default(),binding["name"].as_str().unwrap_or("Assignment")).clicked() { app.rf.draft=binding.clone(); app.rf.previous_name=binding["name"].as_str().unwrap_or_default().into(); app.rf.assignment_control=None; }
                                ui.label(egui::RichText::new(format!("{} · {}",binding["match"]["rf_code"],binding["match"]["gesture"].as_str().unwrap_or_default())).weak());
                            });
                            for action in array(&binding["actions"]) {ui.label(action_summary(&action,&catalog));}
                            ui.label(egui::RichText::new(execution_label(&binding)).small().weak());
                            }); });
                        }
                        ui.separator();
                        egui::Grid::new("rf_binding_fields").num_columns(2).spacing([16.0,6.0]).show(ui,|ui| {
                            text(ui,"Name",&mut app.rf.draft,"name");
                            learned_selector(ui,&mut app.rf.draft["match"],&catalog);
                            choices(ui,"Gesture",&mut app.rf.draft["match"],"gesture",&options(&catalog["gestures"]));
                            number(ui,"Cooldown (ms)",&mut app.rf.draft,"cooldown_ms",1,3_600_000);
                        });
                        ui.checkbox(&mut app.rf.manual_signal,"Enter signal manually (advanced)");
                        if app.rf.manual_signal { egui::Grid::new("rf_manual_signal").num_columns(2).show(ui,|ui| {
                            for (label,field,min,max) in [("Code","rf_code",1,u32::MAX as u64),("Bits","rf_bits",1,32),("Protocol","rf_protocol",1,12)] { number(ui,label,&mut app.rf.draft["match"],field,min,max); }
                        }); }
                        let mut remove=None;
                        if let Some(actions)=app.rf.draft["actions"].as_array_mut() {
                            for (i,action) in actions.iter_mut().enumerate() {
                                ui.push_id(i,|ui| {
                                    egui::Frame::group(ui.style()).inner_margin(10.0).show(ui,|ui| {
                                        ui.horizontal(|ui| { ui.strong(format!("Action {}",i+1)); if actions_len_button(ui).clicked() { remove=Some(i); } });
                                        action_editor(ui,action,&scoped_action_catalog(&catalog,app.rf.assignment_control.as_deref()));
                                    });
                                }); ui.add_space(6.0);
                            }
                            if let Some(i)=remove { actions.remove(i); }
                            if actions.len()<8 && ui.button(format!("{} Add action",crate::ui::icons::PLUS)).clicked() { actions.push(json!({"type":"control","action_id":""})); }
                        }
                        if array(&app.rf.draft["actions"]).iter().any(|a|a["type"]=="virtual-key") {
                            ui.checkbox(&mut app.rf.allow_keyboard,format!("Allow assigned keyboard keys on {}",catalog["hostname"].as_str().unwrap_or("controller host")));
                        }
                        if array(&catalog["entries"]).iter().any(|entry| entry["code"]==app.rf.draft["match"]["rf_code"] && entry["action_kind"].as_u64().unwrap_or(0)>0) {
                            ui.colored_label(ui.visuals().warn_fg_color,"This button also has a board action. Unassign it in Remotes to replace it.");
                        }
                        ui.separator();
                        ui.horizontal(|ui| {
                            if ui.add_enabled(!app.rf.pending && !app.rf.draft["name"].as_str().unwrap_or_default().trim().is_empty() && app.rf.draft["match"]["rf_code"].as_u64().unwrap_or(0)>0 && !array(&app.rf.draft["actions"]).is_empty() && array(&app.rf.draft["actions"]).iter().all(|action|action_ready(action,&catalog)),egui::Button::new(format!("{} Save assignment",crate::ui::icons::FLOPPY_DISK))).clicked() {
                                request(app,"binding.put",json!({"binding":app.rf.draft,"previous_name":app.rf.previous_name,"allow_keyboard":app.rf.allow_keyboard}));
                            }
                            if !app.rf.previous_name.is_empty() && ui.add_enabled(!app.rf.pending,egui::Button::new(format!("{} Remove",crate::ui::icons::TRASH))).clicked() { request(app,"binding.remove",json!({"name":app.rf.previous_name})); }
                        });
                    },
                    1=> {
                        ui.horizontal(|ui| {
                            let learning=catalog["learning"]["active"].as_bool().unwrap_or(false);
                            if learning { ui.colored_label(ui.visuals().selection.bg_fill,"Learning…"); if ui.button("Stop learning").clicked() { request(app,"learn.cancel",json!({})); } }
                            else if ui.add_enabled(catalog["connected"]==true && live_connected && !app.rf.pending,egui::Button::new(format!("{} Learn buttons",crate::ui::icons::PLUS))).clicked() { request(app,"learn.start",json!({"mode":"timer","timeout_ms":30000})); }
                        });
                        if array(&catalog["entries"]).is_empty() {
                            ui.label(egui::RichText::new("No learned RF buttons. Refresh to read the board or learn a new button.").weak());
                        }
                        let mut entries=array(&catalog["entries"]);
                        entries.sort_by_key(|entry|std::cmp::Reverse(entry["last_seen"].as_str().and_then(|time|time.parse::<jiff::Timestamp>().ok())));
                        for entry in entries {
                            ui.push_id(entry["id"].as_u64().unwrap_or_default(),|ui| { egui::Frame::group(ui.style()).show(ui,|ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.strong(entry["name"].as_str().filter(|s|!s.is_empty()).unwrap_or("RF button"));
                                    ui.monospace(entry["code_display"].as_str().unwrap_or_default());
                                    if ui.button("On board…").on_hover_text("Runs without Pealayer or PCController for supported board actions").clicked() {app.rf.board_map=edit_board_map(&entry);}
                                    if ui.button("On controller…").on_hover_text("Percent, gesture, effect and application assignments").clicked() {
                                        let existing=array(&catalog["bindings"]).into_iter().find(|binding| binding["match"]["rf_code"]==entry["code"] && binding["match"]["rf_bits"]==entry["bits"] && binding["match"]["rf_protocol"]==entry["protocol"] && binding["match"]["gesture"]=="down");
                                        app.rf.previous_name=existing.as_ref().and_then(|binding|binding["name"].as_str()).unwrap_or_default().into();
                                        app.rf.draft=existing.unwrap_or_else(|| { let mut binding=new_binding(); binding["name"]=json!(format!("RF {}",entry["id"])); binding["match"]["rf_code"]=entry["code"].clone(); binding["match"]["rf_bits"]=entry["bits"].clone(); binding["match"]["rf_protocol"]=entry["protocol"].clone(); binding });
                                        app.rf.tab=0;
                                        app.rf.assignment_control=None;
                                    }
                                    if ui.add_enabled(!app.rf.pending,egui::Button::new("Transmit")).clicked() { request(app,"transmit",json!({"code":entry["code"],"bits":entry["bits"],"protocol":entry["protocol"],"pulse_us":entry["pulse_us"],"repeats":1})); }
                                    ui.menu_button(crate::ui::icons::DOTS_THREE,|ui| {
                                        if ui.button("Unassign board action").clicked() { request(app,"map",json!({"id":entry["id"],"action":"none"})); ui.close(); }
                                        if ui.button("Edit board action…").clicked() { app.rf.board_map=edit_board_map(&entry); ui.close(); }
                                        if ui.button("Remove learned button").clicked() { request(app,"remove",json!({"id":entry["id"]})); ui.close(); }
                                    });
                                });
                                ui.label(format!("On board · {}",board_assignment(&entry)));
                                for binding in array(&catalog["bindings"]).iter().filter(|binding|signal_matches(binding,&entry)) {
                                    let enabled=if binding["enabled"]==false {" · disabled"} else {""};
                                    ui.label(format!("{} · {}{enabled}",binding["name"].as_str().unwrap_or("Assignment"),binding["match"]["gesture"].as_str().unwrap_or_default()));
                                    for action in array(&binding["actions"]) {ui.label(action_summary(&action,&catalog));}
                                    ui.label(egui::RichText::new(execution_label(binding)).small().weak());
                                }
                                ui.label(format!("Last seen · {}",seen_label(&entry)));
                                ui.label(egui::RichText::new(format!("{} bits · Protocol {} · {} µs",entry["bits"],entry["protocol"],entry["pulse_us"])).small().weak());
                            }); });
                        }
                        ui.separator(); ui.strong("On-board assignment");
                        ui.label("Saved on the board. Supported actions keep working when the computer is off.");
                        ui.label(egui::RichText::new("Percent increase/decrease and application actions need PCController; use On controller for those.").small().weak());
                        egui::Grid::new("rf_board_map").num_columns(2).show(ui,|ui| {
                            ui.label("Learned button");
                            let caption=array(&catalog["entries"]).iter().find(|entry|entry["id"]==app.rf.board_map["id"]).map(|entry|format!("RF {} · {}",entry["id"],entry["code_display"].as_str().unwrap_or_default())).unwrap_or_else(||"Choose…".into());
                            crate::ui::dropdown::ComboBox::from_id_salt("board_learned_button").width(field_width(ui)).selected_text(caption).truncate().show_ui(ui,|ui| {
                                for entry in array(&catalog["entries"]) { if ui.dropdown_choice(app.rf.board_map["id"]==entry["id"],format!("RF {} · {}",entry["id"],entry["code_display"].as_str().unwrap_or_default())).clicked() {app.rf.board_map=edit_board_map(&entry);} }
                            }); ui.end_row();
                            let previous=app.rf.board_map["action"].clone();
                            choices(ui,"Action",&mut app.rf.board_map,"action",&options(&catalog["board_mapping_actions"]));
                            if previous!=app.rf.board_map["action"] {app.rf.board_map.as_object_mut().unwrap().remove("target");app.rf.board_map.as_object_mut().unwrap().remove("behavior");}
                            if let Some(option)=array(&catalog["board_mapping_options"]).iter().find(|option|option["action"]==app.rf.board_map["action"]) {
                                if !array(&option["targets"]).is_empty() { choices(ui,"Target",&mut app.rf.board_map,"target",&target_options(&catalog,option)); }
                                if !array(&option["behaviors"]).is_empty() { choices(ui,"Behavior",&mut app.rf.board_map,"behavior",&options(&option["behaviors"])); }
                            }
                        });
                        if ui.add_enabled(!app.rf.pending && catalog["connected"]==true && live_connected && board_map_valid(&app.rf.board_map,&catalog),egui::Button::new("Save on board")).clicked() {
                            let mut map=app.rf.board_map.clone(); if map["action"]=="none" { map.as_object_mut().unwrap().remove("target"); }
                            if map["action"]=="none" || map["action"]=="menu" { map.as_object_mut().unwrap().remove("behavior"); }
                            request(app,"map",map);
                        }
                    },
                    2=> {
                        crate::ui::dropdown::ComboBox::from_id_salt("transmit_learned_signal").selected_text("Use a learned signal…").width(field_width(ui)).show_ui(ui,|ui| {
                            for entry in array(&catalog["entries"]) {
                                if ui.dropdown_choice(false,format!("RF {} · {}",entry["id"],entry["code_display"].as_str().unwrap_or_default())).clicked() {
                                    app.rf.code=entry["code"].to_string();
                                    app.rf.bits=entry["bits"].as_u64().unwrap_or(24) as u8;
                                    app.rf.protocol=entry["protocol"].as_u64().unwrap_or(1) as u8;
                                    app.rf.pulse_us=entry["pulse_us"].as_u64().unwrap_or(0) as u16;
                                }
                            }
                        });
                        egui::Grid::new("rf_tx").num_columns(2).spacing([16.0,8.0]).show(ui,|ui| {
                            ui.label("Code (hex or decimal)"); ui.text_edit_singleline(&mut app.rf.code); ui.end_row();
                            ui.label("Bits"); ui.add(egui::DragValue::new(&mut app.rf.bits).range(1..=32)); ui.end_row();
                            ui.label("Protocol"); ui.add(egui::DragValue::new(&mut app.rf.protocol).range(1..=12)); ui.end_row();
                            ui.label("Pulse (µs, 0 = protocol default)"); ui.add(egui::DragValue::new(&mut app.rf.pulse_us)); ui.end_row();
                            ui.label("Repeats"); ui.add(egui::DragValue::new(&mut app.rf.repeats).range(1..=20)); ui.end_row();
                        });
                        let code=app.rf.code.trim(); let parsed=code.strip_prefix("0x").or_else(||code.strip_prefix("0X")).map(|s|u32::from_str_radix(s,16)).unwrap_or_else(||code.parse::<u32>()).ok().filter(|v|*v!=0);
                        if ui.add_enabled(parsed.is_some() && !app.rf.pending && catalog["connected"]==true && live_connected,egui::Button::new(format!("{} Transmit",crate::ui::icons::PAPER_PLANE_TILT))).clicked() { request(app,"transmit",json!({"code":parsed,"bits":app.rf.bits,"protocol":app.rf.protocol,"pulse_us":app.rf.pulse_us,"repeats":app.rf.repeats})); }
                    },
                    _=> {
                        for event in array(&catalog["activity"]) { ui.label(event["text"].as_str().unwrap_or_default()); ui.label(egui::RichText::new(event["time"].as_str().unwrap_or_default()).small().weak()); ui.separator(); }
                    }
                }
            });
        });
    app.rf.open = open;
    if !open { app.rf.assignment_control = None; }
}

fn actions_len_button(ui: &mut egui::Ui) -> egui::Response {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.button(crate::ui::icons::TRASH)
            .on_hover_text("Remove action")
    })
    .inner
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn card_assignment_opens_an_unbound_scoped_draft_without_mutating_catalog() {
        let mut state = RfState::default();
        state.catalog = json!({"bindings":[{"name":"Existing motion","actions":[{"action_id":"seat.a.down"}]}],
            "peripherals":{"controls":[{"key":"seat.a","actions":[{"id":"seat.a.down"}]},
                {"key":"pwm.1","actions":[{"id":"pwm.1.on"}]}]}});
        let previous = state.catalog.clone();
        state.begin_control_assignment("pwm.1", "Light");
        assert!(state.open);
        assert_eq!(state.draft["name"], "Light");
        assert_eq!(state.draft["actions"][0]["action_id"], "");
        assert_eq!(state.draft["match"]["rf_code"], 0);
        assert_eq!(state.catalog, previous);
        let scoped = scoped_action_catalog(&state.catalog, state.assignment_control.as_deref());
        assert_eq!(control_options(&scoped).len(), 1);
        assert_eq!(control_options(&scoped)[0].0, "pwm.1.on");
        assert_eq!(state.catalog, previous);
        assert!(control_options(&scoped_action_catalog(&state.catalog, Some("missing"))).is_empty());
    }

    #[test]
    fn shallow_catalog_refresh_preserves_authoritative_board_entries() {
        let mut state = RfState::default();
        state.apply_catalog(json!({
            "connected": true,
            "entries_sampled": true,
            "entries": [{"id": 0, "code": 42}]
        }));

        state.apply_catalog(json!({"connected": true, "bindings": []}));

        assert_eq!(state.catalog["entries"][0]["code"], 42);
        assert_eq!(state.catalog["entries_sampled"], true);
    }

    #[test]
    fn authoritative_catalog_replaces_board_entries() {
        let mut state = RfState::default();
        state.apply_catalog(json!({
            "connected": true,
            "entries_sampled": true,
            "entries": [{"id": 0, "code": 42}]
        }));

        state.apply_catalog(json!({
            "connected": true,
            "entries_sampled": true,
            "entries": [{"id": 1, "code": 99}]
        }));

        assert_eq!(state.catalog["entries"].as_array().map(Vec::len), Some(1));
        assert_eq!(state.catalog["entries"][0]["code"], 99);
    }

    #[test]
    fn genuine_receive_updates_matching_signal_without_mutation_or_date_regression() {
        let mut state = RfState::default();
        state.apply_catalog(json!({"connected":true,"entries_sampled":true,"entries":[
            {"id":0,"code":42,"bits":24,"protocol":1},
            {"id":1,"code":42,"bits":16,"protocol":1}
        ]}));
        let event = json!({"id":1,"kind":"rf.receive","source":"board","rf_code":42,"rf_bits":24,"rf_protocol":1,"time":"2026-10-13T10:00:00Z"});
        state.receive(event.clone());
        assert_eq!(state.catalog["entries"][0]["last_seen"], event["time"]);
        assert!(state.catalog["entries"][1]["last_seen"].is_null());
        let mut older = event.clone();
        older["time"] = json!("2026-10-13T09:00:00Z");
        state.receive(older);
        assert_eq!(state.catalog["entries"][0]["last_seen"], event["time"]);
        let mut tx = event.clone();
        tx["kind"] = json!("rf.transmit");
        tx["time"] = json!("2026-10-13T11:00:00Z");
        state.receive(tx);
        assert_eq!(state.catalog["entries"][0]["last_seen"], event["time"]);
        let mut other = event.clone();
        other["rf_code"] = json!(43);
        other["time"] = json!("2026-10-13T10:03:00Z");
        state.receive(other.clone());
        let mut delayed = event.clone();
        delayed["time"] = json!("2026-10-13T09:00:00Z");
        state.receive(delayed);
        assert_eq!(last_received(&state.catalog).unwrap()["rf_code"], 43);
        assert_eq!(state.catalog["activity"][0]["time"], other["time"]);
        assert!(!state.pending);
        assert!(state.last_result.is_null());
    }

    #[test]
    fn waveform_identity_and_advertised_action_summaries() {
        let entry = json!({"code":42,"bits":24,"protocol":1});
        let binding = json!({"match":{"rf_code":42,"rf_bits":24,"rf_protocol":1}});
        assert!(signal_matches(&binding, &entry));
        assert!(!signal_matches(
            &binding,
            &json!({"code":42,"bits":16,"protocol":1})
        ));
        let catalog = json!({"peripherals":{"controls":[
            {"key":"pwm.0","name":"Window","step_percent":5,"actions":[{"id":"pwm.0.increase","verb":"increase","name":"Increase"}]},
            {"key":"pwm.1","name":"Unavailable"}
        ]}});
        assert_eq!(control_options(&catalog).len(), 1);
        assert_eq!(
            action_summary(
                &json!({"type":"control","action_id":"pwm.0.increase"}),
                &catalog
            ),
            "Window · Increase (5%)"
        );
        assert!(
            action_summary(&json!({"type":"control","action_id":"pwm.1.on"}), &catalog)
                .starts_with("Unavailable")
        );
    }

    #[test]
    fn drawing_advertised_peer_action_preserves_its_parameter() {
        let context = egui::Context::default();
        let mut action = json!({"type":"app","app_target":"peer","app_kind":"peer.choose","app_value":"kept exactly"});
        let catalog = json!({"action_types":["app"],"applications":[{"id":"peer","surface":"peer","values":{"app_actions":"peer.choose"}}]});
        for width in [480.0, 820.0] {
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 500.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    action_editor(ui, &mut action, &catalog);
                },
            );
            output.textures_delta.clear();
            assert_eq!(action["app_value"], "kept exactly");
        }
    }

    #[test]
    fn editing_board_mapping_preserves_semantics_and_validates_choices() {
        let entry = json!({"id":5,"board_assignment":{"action":"pwm","target":"0","behavior":"toggle","label":"Window · Toggle"}});
        let map = edit_board_map(&entry);
        assert_eq!(
            map,
            json!({"id":5,"action":"pwm","target":"0","behavior":"toggle"})
        );
        let catalog = json!({"entries":[entry],"board_mapping_options":[{"action":"pwm","targets":["0"],"behaviors":["toggle"]}]});
        assert!(board_map_valid(&map, &catalog));
        let unknown = edit_board_map(&json!({"id":5}));
        assert!(!board_map_valid(
            &unknown,
            &json!({"entries":[{"id":5}],"board_mapping_options":[{"action":"none","targets":[],"behaviors":[]}]})
        ));
        assert!(!board_map_valid(
            &json!({"id":5,"action":"pwm","target":"0"}),
            &catalog
        ));
        assert!(!board_map_valid(
            &json!({"id":9,"action":"pwm","target":"0","behavior":"toggle"}),
            &catalog
        ));
    }

    #[test]
    fn refresh_retains_reception_date_and_structured_mutation_error() {
        let mut state = RfState::default();
        state.apply_catalog(json!({"connected":true,"entries_sampled":true,"entries":[{"id":5,"last_seen":"2026-10-13T10:00:00Z"}]}));
        state.apply_catalog(json!({"connected":true,"bindings":[]}));
        assert_eq!(
            state.catalog["entries"][0]["last_seen"],
            "2026-10-13T10:00:00Z"
        );
        state.error_detail = json!({"kind":"resource_busy","resource":"board","retryable":true});
        state.complete("rf-map", Err("The board is busy".into()));
        state.complete("rf-catalog", Ok(json!({"connected":true,"bindings":[]})));
        assert_eq!(state.error, "The board is busy");
        assert_eq!(state.snapshot()["error_detail"]["kind"], "resource_busy");
        assert_eq!(
            seen_label(&json!({"last_seen":null})),
            "Not seen in this controller session"
        );
    }
}
