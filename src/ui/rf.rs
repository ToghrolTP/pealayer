use crate::app::PealayerApp;
use crate::ui::dropdown::DropdownUiExt;
use eframe::egui;
use serde_json::{Value, json};
use std::time::Duration;
use std::collections::BTreeMap;

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
    "preferences.set",
    "observed.add",
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
    raise_on_open: bool,
    assignment_sequence: u64,
    session_open: bool,
    reception_baseline: BTreeMap<String,u64>,
    preferences_draft: Value,
    assignment_baseline: Value,
    board_baseline: Value,
    pending_draft: Option<Value>,
    confirmation: Option<RfIntent>,
}

#[derive(Clone)]
enum RfIntent {
    Close,
    Assignment { draft: Value, previous_name: String, control: Option<String> },
    Board(Value),
    Remove { operation: &'static str, params: Value },
}

fn new_binding() -> Value {
    json!({"name":"", "enabled":true,"cooldown_ms":250,
        "match":{"kind":"rf.gesture","source":"rf","rf_code":0,"rf_bits":24,"rf_protocol":1,"gesture":"down"},
        "actions":[{"type":"control","action_id":""}]})
}

fn utf8_prefix(value: &str, max_bytes: usize) -> String {
    value.chars().scan(0, |bytes, character| {
        *bytes += character.len_utf8(); (*bytes <= max_bytes).then_some(character)
    }).collect()
}

fn unique_binding_name(preferred: &str, catalog: &Value) -> String {
    let base = if preferred.trim().is_empty() { "RF assignment" } else { preferred.trim() };
    let names = array(&catalog["bindings"]).into_iter().filter_map(|binding|
        binding["name"].as_str().map(|name| name.trim().to_lowercase())).collect::<std::collections::BTreeSet<_>>();
    for index in 0..=names.len() + 1 {
        let suffix = if index == 0 { String::new() } else { format!(" {}", index + 1) };
        let candidate = format!("{}{}", utf8_prefix(base, 64 - suffix.len()).trim_end(), suffix);
        if !names.contains(&candidate.to_lowercase()) { return candidate; }
    }
    unreachable!("more candidates than existing names")
}

fn binding_name_conflicts(name: &str, previous_name: &str, catalog: &Value) -> bool {
    array(&catalog["bindings"]).iter().any(|binding| binding["name"].as_str().is_some_and(|existing|
        existing.to_lowercase() == name.trim().to_lowercase()
            && (previous_name.is_empty() || existing.to_lowercase() != previous_name.to_lowercase())))
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
            raise_on_open: false,
            assignment_sequence: 0,
            session_open: false,
            reception_baseline: BTreeMap::new(),
            preferences_draft: Value::Null,
            assignment_baseline: new_binding(),
            board_baseline: json!({"id":null,"action":""}),
            pending_draft: None,
            confirmation: None,
        }
    }
}

impl RfState {
    fn assignment_dirty(&self) -> bool { self.draft != self.assignment_baseline }
    fn board_dirty(&self) -> bool { self.board_map != self.board_baseline }
    pub(crate) fn dirty(&self) -> bool {
        self.assignment_dirty() || self.board_dirty()
            || (self.preferences_draft.is_object() && self.catalog["preferences"].is_object()
                && self.preferences_draft != self.catalog["preferences"])
    }
    fn propose(&mut self, intent: RfIntent) {
        if self.pending || self.confirmation.is_some() { return; }
        let confirm = match &intent {
            RfIntent::Close => self.dirty(),
            RfIntent::Assignment { .. } => self.assignment_dirty(),
            RfIntent::Board(_) => self.board_dirty(),
            RfIntent::Remove { .. } => true,
        };
        if confirm { self.confirmation = Some(intent); }
        else { self.apply_intent(intent); }
    }
    fn apply_intent(&mut self, intent: RfIntent) {
        match intent {
            RfIntent::Close => {
                self.draft = self.assignment_baseline.clone();
                self.board_map = self.board_baseline.clone();
                self.preferences_draft = self.catalog["preferences"].clone();
                self.open = false; self.assignment_control = None; self.session_open = false;
            }
            RfIntent::Assignment { draft, previous_name, control } => {
                self.assignment_baseline = draft.clone(); self.draft = draft;
                self.previous_name = previous_name; self.assignment_control = control;
                self.allow_keyboard = false; self.tab = 0;
                self.assignment_sequence = self.assignment_sequence.wrapping_add(1);
            }
            RfIntent::Board(map) => {
                self.board_baseline = map.clone(); self.board_map = map; self.tab = 1;
            }
            RfIntent::Remove { .. } => {} // Only dispatched explicitly after confirmation.
        }
    }
    fn begin_reception_session(&mut self) {
        if self.session_open { return; }
        self.session_open = true;
        self.reception_baseline.clear();
        for receipt in array(&self.catalog["receptions"]) {
            self.reception_baseline.insert(signal_key(&receipt),receipt["received_count"].as_u64().unwrap_or_default());
        }
    }
    fn begin_control_assignment(&mut self, key: &str, name: &str) {
        self.open = true;
        let mut draft = new_binding();
        draft["name"] = json!(unique_binding_name(name, &self.catalog));
        self.propose(RfIntent::Assignment { draft, previous_name: String::new(), control: Some(key.to_owned()) });
        self.raise_on_open = true;
    }
    pub(crate) fn receive(&mut self, event: Value) {
        if event["kind"] != "rf.receive" || event["source"] != "board" {
            return;
        }
        if !valid_signal(&json!({"code":event["rf_code"],"bits":event["rf_bits"],"protocol":event["rf_protocol"]})) { return; }
        let Some(time) = event["time"]
            .as_str()
            .and_then(|time| time.parse::<jiff::Timestamp>().ok())
        else {
            return;
        };
        if self.open { self.begin_reception_session(); }
        if let Some(generation)=event["metadata"]["connection_generation"].as_str().and_then(|v|v.parse::<u64>().ok()) {
            if let Some(previous)=self.catalog["connection_generation"].as_u64() {
                if generation<previous { return; }
                if generation!=previous {
                    self.catalog["receptions"]=json!([]);self.reception_baseline.clear();
                    self.catalog["entries"]=json!([]);self.catalog["entries_sampled"]=json!(false);
                }
            }
            self.catalog["connection_generation"]=json!(generation);
        }
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
        let receipt = json!({"code":event["rf_code"],"bits":event["rf_bits"],"protocol":event["rf_protocol"],
            "pulse_us":event["rf_pulse_us"],"last_seen":event["time"],
            "received_count":event["metadata"]["received_count"].as_str().and_then(|v|v.parse::<u64>().ok()).unwrap_or_default(),
            "connection_generation":event["metadata"]["connection_generation"].as_str().and_then(|v|v.parse::<u64>().ok()).unwrap_or_default()});
        if !valid_signal(&receipt) { return; }
        if !self.catalog["receptions"].is_array() { self.catalog["receptions"]=json!([]); }
        let receipts=self.catalog["receptions"].as_array_mut().unwrap();
        if let Some(old)=receipts.iter_mut().find(|old|signal_key(old)==signal_key(&receipt)) { *old=receipt; }
        else {receipts.insert(0,receipt);receipts.truncate(256);}
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
        let replaced=(self.catalog["host_instance_id"].as_str().is_some() && value["host_instance_id"].as_str().is_some() && value["host_instance_id"]!=self.catalog["host_instance_id"])
            || (self.catalog["connection_generation"].as_u64().is_some() && value["connection_generation"].as_u64().is_some() && value["connection_generation"]!=self.catalog["connection_generation"]);
        if replaced {
            self.reception_baseline.clear();
            for receipt in array(&value["receptions"]) {
                self.reception_baseline.insert(signal_key(&receipt),receipt["received_count"].as_u64().unwrap_or_default());
            }
        } else if let Some(receipts)=value["receptions"].as_array_mut() {
            for old in array(&self.catalog["receptions"]) {
                if let Some(receipt)=receipts.iter_mut().find(|r|signal_key(r)==signal_key(&old)) {
                    if old["received_count"].as_u64() > receipt["received_count"].as_u64() { *receipt=old; }
                } else {receipts.push(old);}
            }
        }
        if self.preferences_draft.is_null() || self.preferences_draft==self.catalog["preferences"] { self.preferences_draft=value["preferences"].clone(); }
        if !replaced && value["connected"].as_bool() == Some(true)
            && value["entries_sampled"].as_bool() != Some(true)
        {
            value["entries"] = self.catalog["entries"].clone();
            value["entries_sampled"] = self.catalog["entries_sampled"].clone();
        }
        if !replaced && value["hostname"] == self.catalog["hostname"]
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
                    if let Some(submitted) = self.pending_draft.take() {
                        self.assignment_baseline = submitted;
                    }
                }
                if operation == "rf-binding.remove" && self.previous_name == self.pending_name {
                    self.previous_name.clear();
                    self.apply_intent(RfIntent::Assignment { draft: new_binding(), previous_name: String::new(), control: self.assignment_control.clone() });
                }
                if matches!(operation, "rf-binding.put" | "rf-binding.remove") && value.is_array() {
                    self.catalog["bindings"] = value.clone();
                }
                self.last_operation = operation.into();
                self.last_result = value;
                if operation=="rf-preferences.set" {
                    self.catalog["preferences"]=self.last_result.clone();
                    self.preferences_draft=self.last_result.clone();
                }
                if operation=="rf-map" && let Some(submitted)=self.pending_draft.take() { self.board_baseline=submitted; }
                self.pending_draft=None;
                true
            }
            Err(error) => {
                self.pending_draft=None;
                self.error = error;
                self.error_is_catalog = operation == "rf-catalog";
                false
            }
        }
    }
}

fn signal_key(entry: &Value) -> String {format!("{}:{}:{}",entry["code"],entry["bits"],entry["protocol"])}

fn valid_signal(entry: &Value) -> bool {
    entry["code"].as_u64().is_some_and(|code| code > 0 && code <= u32::MAX as u64)
        && entry["bits"].as_u64().is_some_and(|bits| (1..=32).contains(&bits))
        && entry["protocol"].as_u64().is_some_and(|protocol| (1..=12).contains(&protocol))
}

fn known_signals(catalog: &Value) -> Vec<Value> {
    let mut rows=array(&catalog["entries"]);
    for receipt in array(&catalog["receptions"]) {
        if !rows.iter().any(|entry|signal_key(entry)==signal_key(&receipt)) {
            let mut row=receipt;row["code_display"]=json!(format!("0x{:08X}",row["code"].as_u64().unwrap_or_default()));rows.push(row);
        }
    }
    for binding in array(&catalog["bindings"]) {
        let signal=json!({"code":binding["match"]["rf_code"],"bits":binding["match"]["rf_bits"],"protocol":binding["match"]["rf_protocol"]});
        if !rows.iter().any(|entry|signal_key(entry)==signal_key(&signal)) {
            let mut row=signal;row["code_display"]=json!(format!("0x{:08X}",row["code"].as_u64().unwrap_or_default()));rows.push(row);
        }
    }
    rows.retain(valid_signal);
    rows
}

fn reception_status(state: &RfState, entry: &Value, connected: bool) -> (bool,u64,Option<Duration>) {
    let receipt=array(&state.catalog["receptions"]).into_iter().find(|r|signal_key(r)==signal_key(entry));
    let Some(receipt)=receipt else{return(false,0,None)};
    let count=receipt["received_count"].as_u64().unwrap_or_default().saturating_sub(*state.reception_baseline.get(&signal_key(entry)).unwrap_or(&0));
    let age=receipt["last_seen"].as_str().and_then(|v|v.parse::<jiff::Timestamp>().ok())
        .map(|seen|(jiff::Timestamp::now().as_millisecond()-seen.as_millisecond()).max(0) as u64);
    let remaining=age.filter(|age|*age<250 && connected).map(|age|Duration::from_millis(250-age));
    (remaining.is_some(),count,remaining)
}

fn draw_reception(state: &RfState, ui: &mut egui::Ui, entry: &Value, connected: bool, language: crate::config::AppLanguage) {
    let (active,count,remaining)=reception_status(state,entry,connected);
    let color=if active {egui::Color32::from_rgb(45,190,100)} else {ui.visuals().weak_text_color()};
    let (rect,response)=ui.allocate_exact_size(egui::vec2(14.0,14.0),egui::Sense::hover());
    ui.painter().circle_filled(rect.center(),4.0,color);
    response.on_hover_text(crate::ui::i18n::tr(language,if active {"Receiving"} else {"Not receiving"}));
    ui.label(egui::RichText::new(format!("×{count}")).monospace().small().weak()).on_hover_text(crate::ui::i18n::tr(language,"Receptions since this dialog opened"));
    if let Some(remaining)=remaining {ui.ctx().request_repaint_after(remaining);}
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
        let submitted = match operation {
            "binding.put" if params["binding"] == self.rf.draft => Some(params["binding"].clone()),
            "map" if params["id"] == self.rf.board_map["id"] && params["action"] == self.rf.board_map["action"] => Some(self.rf.board_map.clone()),
            _ => None,
        };
        self.engine_handle.request_controller_call(
            &format!("rf-{operation}"),
            &format!("controller.rf.{operation}"),
            params,
        )?;
        self.rf.pending_name = pending_name;
        self.rf.pending_draft = submitted;
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

fn text(ui: &mut egui::Ui, label: &'static str, value: &mut Value, field: &str) {
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
    label: &'static str,
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

fn rf_tr(ui:&egui::Ui,label:&'static str) -> String {
    let language=ui.data(|data|data.get_temp::<crate::config::AppLanguage>(egui::Id::new("rf-ui-language"))).unwrap_or(crate::config::AppLanguage::English);
    crate::ui::i18n::tr(language,label)
}

fn field_label(ui:&mut egui::Ui,label:&'static str) {
    let height=crate::ui::control_density::height(ui);
    let label=rf_tr(ui,label);
    ui.add_sized(egui::vec2(130.0,height),egui::Label::new(&label).truncate()).on_hover_text(label);
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

fn pwm_board_action(action: &Value) -> Option<(String, &str)> {
    if action["type"] != "board" { return None; }
    let words = action["command"].as_str()?.split_whitespace().collect::<Vec<_>>();
    if words.len() != 3 || words[0] != "pwm"
        || !["on", "off", "toggle", "increase", "decrease", "roll", "remember"].contains(&words[1]) {
        return None;
    }
    let channel = words[2].parse::<u8>().ok().filter(|channel| *channel < 16)?;
    if channel.to_string() != words[2] { return None; }
    Some((format!("pwm.{channel}"), words[1]))
}

fn grouped_bindings(catalog: &Value, key: Option<&str>) -> Vec<Vec<Value>> {
    let controls = array(&catalog["peripherals"]["controls"]);
    let mut groups: Vec<Vec<Value>> = Vec::new();
    for binding in array(&catalog["bindings"]) {
        if let Some(key) = key {
            let targeted = array(&binding["actions"]).iter().any(|action| {
                if action["type"] == "control" {
                    controls.iter().filter(|control| control["key"] == key).any(|control|
                        array(&control["actions"]).iter().any(|advertised|
                            advertised["id"].as_str().is_some_and(|id| !id.is_empty() && action["action_id"] == id)))
                } else { pwm_board_action(action).is_some_and(|(target, _)| target == key) }
            });
            if !targeted { continue; }
        }
        let signal = |binding: &Value| {
            let m = &binding["match"];
            Some((m["rf_code"].as_u64().filter(|n| (1..=u32::MAX as u64).contains(n))?,
                m["rf_bits"].as_u64().filter(|n| (1..=32).contains(n))?,
                m["rf_protocol"].as_u64().filter(|n| (1..=12).contains(n))?))
        };
        let target = signal(&binding).and_then(|identity| groups.iter().position(|group| signal(&group[0]) == Some(identity)));
        if let Some(index) = target { groups[index].push(binding); }
        else { groups.push(vec![binding]); }
    }
    groups
}

fn draw_saved_assignments(app: &mut PealayerApp, ui: &mut egui::Ui, catalog: &Value, live_connected: bool) {
    let groups = grouped_bindings(catalog, app.rf.assignment_control.as_deref());
    ui.separator();
    ui.strong(app.tr(if app.rf.assignment_control.is_some() { "Saved buttons for this channel" } else { "Saved RF buttons" }));
    for group in groups {
        let binding = &group[0];
        let signal = format!("{} · {} bits · Protocol {}", binding["match"]["rf_code"], binding["match"]["rf_bits"], binding["match"]["rf_protocol"]);
        let waveform=json!({"code":binding["match"]["rf_code"],"bits":binding["match"]["rf_bits"],"protocol":binding["match"]["rf_protocol"]});
        ui.push_id(signal_key(&waveform), |ui| {
            let id=ui.make_persistent_id("rf_signal_group");
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(),id,false)
                .show_header(ui,|ui| {draw_reception(&app.rf,ui,&waveform,live_connected,app.language);ui.add(egui::Label::new(format!("{signal} · {}",group.len())).truncate()).on_hover_text(&signal);})
                .body(|ui| {
                for binding in group {
                    ui.push_id(binding["name"].as_str().unwrap_or_default(), |ui| {
                        egui::Frame::group(ui.style()).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let mut enabled = binding["enabled"].as_bool().unwrap_or(false);
                                if ui.add_enabled(live_connected && !app.rf.pending, egui::Checkbox::new(&mut enabled, "")).changed() {
                                    let mut updated = binding.clone(); updated["enabled"] = json!(enabled);
                                    request(app, "binding.put", json!({"binding":updated,"previous_name":binding["name"]}));
                                }
                                let summary=array(&binding["actions"]).iter().map(|action|action_summary(action,catalog)).collect::<Vec<_>>().join(" · ");
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui| {
                                    if rf_icon_button(ui,crate::ui::icons::PENCIL_SIMPLE,&app.tr("Edit assignment"),!app.rf.pending).clicked() {
                                        app.rf.propose(RfIntent::Assignment { draft: binding.clone(), previous_name: binding["name"].as_str().unwrap_or_default().into(), control: app.rf.assignment_control.clone() });
                                    }
                                    let width=ui.available_width().max(1.0);
                                    ui.allocate_ui_with_layout(egui::vec2(width,0.0),egui::Layout::top_down(egui::Align::Min),|ui| {
                                        ui.strong(binding["match"]["gesture"].as_str().unwrap_or_default());
                                        ui.add(egui::Label::new(&summary).wrap());
                                    });
                                });
                            });
                            egui::CollapsingHeader::new(app.tr("Technical details")).show(ui,|ui| {
                                ui.label(binding["name"].as_str().unwrap_or_default());
                                for action in array(&binding["actions"]) { ui.label(action_summary(&action,catalog)); }
                                ui.label(egui::RichText::new(execution_label(&binding)).small().weak());
                            });
                        });
                    });
                }
            });
        });
    }
}

fn rf_icon_button(ui: &mut egui::Ui, icon: &str, tooltip: &str, enabled: bool) -> egui::Response {
    let side=crate::ui::control_density::height(ui).max(ui.text_style_height(&egui::TextStyle::Button)+4.0);
    ui.add_enabled_ui(enabled,|ui| {
        let (rect,response)=ui.allocate_exact_size(egui::Vec2::splat(side),egui::Sense::click());
        let visuals=ui.style().interact(&response);
        ui.painter().rect(rect,visuals.corner_radius,visuals.weak_bg_fill,visuals.bg_stroke,egui::StrokeKind::Inside);
        ui.painter().text(rect.center(),egui::Align2::CENTER_CENTER,icon,egui::TextStyle::Button.resolve(ui.style()),visuals.fg_stroke.color);
        response.widget_info(||egui::WidgetInfo::labeled(egui::WidgetType::Button,response.enabled(),tooltip));
        response.on_hover_text(tooltip)
    }).inner
}

fn behavior_options(option: &Value) -> Vec<(String,String)> {
    options(&option["behaviors"]).into_iter().map(|(id,label)| {
        let caption=option["behavior_labels"][&id].as_str().unwrap_or(&label).to_owned(); (id,caption)
    }).collect()
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
        "board" => pwm_board_action(action).and_then(|(key, verb)| {
            array(&catalog["peripherals"]["controls"]).iter().find(|control| control["key"] == key)
                .and_then(|control| control["name"].as_str()).map(|name| format!("{} · {}", name,
                    match verb { "roll" => "Roll brightness", "remember" => "Remember brightness", _ => verb }))
        }).unwrap_or_else(|| format!("Controller · {}", value("command"))),
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

fn error_caption(detail: &Value) -> Option<&'static str> {
    if detail["kind"] == "timeout" { Some("PCController request timed out") }
    else if detail["kind"] == "connection_unavailable" || detail["transport_failed"] == true {
        Some("PCController connection is unavailable")
    } else { None }
}

fn draw_error(app: &PealayerApp, ui: &mut egui::Ui, message: &str, detail: &Value) {
    if message.is_empty() {
        return;
    }
    egui::Frame::group(ui.style()).show(ui, |ui| {
        let headline = error_caption(detail).map(|caption| app.tr(caption)).unwrap_or_else(|| message.into());
        ui.colored_label(ui.visuals().error_fg_color, headline);
        if detail["transport_failed"] == true {
            ui.label(app.tr("Check the connection and current state before retrying"));
        } else if detail["retryable"] == true {
            ui.label(app.tr("You can retry when the controller is ready."));
        }
        if detail.is_object() {
            ui.collapsing(app.tr("Details"), |ui| {
                ui.add(egui::Label::new(message).wrap());
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
    field_label(ui,"RF code");
    let entries = known_signals(catalog);
    let caption = entries
        .iter()
        .find(|entry| {
            signal["rf_code"] == entry["code"]
                && signal["rf_bits"] == entry["bits"]
                && signal["rf_protocol"] == entry["protocol"]
        })
        .map(|entry| {
            format!(
                "{} · {}",
                entry["name"].as_str().filter(|v|!v.is_empty()).unwrap_or("RF"),
                entry["code_display"].as_str().unwrap_or_default()
            )
        })
        .unwrap_or_else(|| "Choose an RF code…".into());
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
                            "{} · {}",
                            entry["name"].as_str().filter(|v|!v.is_empty()).unwrap_or("RF"),
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
fn number(ui: &mut egui::Ui, label: &'static str, value: &mut Value, field: &str, min: u64, max: u64) {
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
        app.rf.session_open=false;
        return;
    }
    app.rf.begin_reception_session();
    ui.ctx().data_mut(|data|data.insert_temp(egui::Id::new("rf-ui-language"),app.language));
    let live_connected = app
        .engine_handle
        .is_connected
        .load(std::sync::atomic::Ordering::Acquire);
    if app.rf.pending {
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
    let window = egui::Window::new(format!("{} RF controls",crate::ui::icons::RADIO)).id(egui::Id::new("rf_manager"))
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
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui| {
                    if ui.add_enabled_ui(!app.rf.pending,|ui|crate::ui::dialog::close_button(ui,&app.tr("Close RF controls"))).inner.clicked() {open=false;}
                    if rf_icon_button(ui,crate::ui::icons::ARROW_CLOCKWISE,&app.tr("Refresh learned buttons"),!app.rf.pending).clicked() {request(app,"catalog",json!({"read_board":true}));}
                    if app.rf.pending {ui.spinner();}
                    let width=ui.available_width().max(1.0);
                    ui.allocate_ui_with_layout(egui::vec2(width,crate::ui::dialog::CLOSE_BUTTON_SIDE),egui::Layout::left_to_right(egui::Align::Center),|ui| {
                        ui.add(egui::Label::new(egui::RichText::new(format!("{} {}",crate::ui::icons::RADIO,app.tr("RF controls"))).strong()).truncate());
                    });
                });
            });
            ui.horizontal_wrapped(|ui| {
                for (i,caption) in ["Assignments","Remotes","Transmit","Activity","Settings"].iter().enumerate() { let caption=app.tr(caption);ui.dropdown_value(&mut app.rf.tab,i,caption); }
            });
            ui.separator();
            draw_error(app,ui,&app.rf.error,&app.rf.error_detail);
            if let Some(error)=catalog["board_error"].as_str() { ui.colored_label(ui.visuals().warn_fg_color,error); }
            egui::ScrollArea::vertical().id_salt(("rf_body",app.rf.assignment_sequence)).auto_shrink([false,false]).show(ui,|ui| {
                ui.set_min_width(ui.available_width());
                ui.add_enabled_ui(!app.rf.pending,|ui| { match app.rf.tab {
                    0=> {
                        ui.horizontal_wrapped(|ui| {
                            if ui.button(format!("{} {}",crate::ui::icons::PLUS,app.tr("New assignment"))).clicked() { app.rf.propose(RfIntent::Assignment { draft: new_binding(), previous_name: String::new(), control: app.rf.assignment_control.clone() }); }
                            if let Some(event)=last_received(&catalog) {
                                if ui.button(app.tr("Use last received")).clicked() {
                                    for (to,from) in [("rf_code","rf_code"),("rf_bits","rf_bits"),("rf_protocol","rf_protocol")] { app.rf.draft["match"][to]=event[from].clone(); }
                                }
                            }
                        });
                        ui.separator();
                        egui::Grid::new("rf_binding_fields").num_columns(2).spacing([16.0,6.0]).show(ui,|ui| {
                            text(ui,"Name",&mut app.rf.draft,"name");
                            learned_selector(ui,&mut app.rf.draft["match"],&catalog);
                            choices(ui,"Gesture",&mut app.rf.draft["match"],"gesture",&options(&catalog["gestures"]));
                            number(ui,"Cooldown (ms)",&mut app.rf.draft,"cooldown_ms",1,3_600_000);
                        });
                        let caption=app.tr("Enter signal manually (advanced)");ui.checkbox(&mut app.rf.manual_signal,caption);
                        if app.rf.manual_signal { egui::Grid::new("rf_manual_signal").num_columns(2).show(ui,|ui| {
                            for (label,field,min,max) in [("Code","rf_code",1,u32::MAX as u64),("Bits","rf_bits",1,32),("Protocol","rf_protocol",1,12)] { number(ui,label,&mut app.rf.draft["match"],field,min,max); }
                        }); }
                        let mut remove=None;
                        if let Some(actions)=app.rf.draft["actions"].as_array_mut() {
                            for (i,action) in actions.iter_mut().enumerate() {
                                ui.push_id(i,|ui| {
                                    egui::Frame::group(ui.style()).inner_margin(10.0).show(ui,|ui| {
                                        ui.horizontal(|ui| { ui.strong(format!("{} {}",rf_tr(ui,"Action"),i+1)); if actions_len_button(ui).clicked() { remove=Some(i); } });
                                        action_editor(ui,action,&scoped_action_catalog(&catalog,app.rf.assignment_control.as_deref()));
                                    });
                                }); ui.add_space(6.0);
                            }
                            if let Some(i)=remove { actions.remove(i); }
                            if actions.len()<8 && ui.button(format!("{} {}",crate::ui::icons::PLUS,rf_tr(ui,"Add action"))).clicked() { actions.push(json!({"type":"control","action_id":""})); }
                        }
                        if array(&app.rf.draft["actions"]).iter().any(|a|a["type"]=="virtual-key") {
                            ui.checkbox(&mut app.rf.allow_keyboard,format!("Allow assigned keyboard keys on {}",catalog["hostname"].as_str().unwrap_or("controller host")));
                        }
                        if array(&catalog["entries"]).iter().any(|entry| signal_matches(&app.rf.draft,entry) && entry["action_kind"].as_u64().unwrap_or(0)>0) {
                            ui.colored_label(ui.visuals().warn_fg_color,app.tr("This button also has a board action. Unassign it in Remotes to replace it."));
                        }
                        ui.separator();
                        ui.horizontal_wrapped(|ui| {
                            let conflict = binding_name_conflicts(app.rf.draft["name"].as_str().unwrap_or_default(), &app.rf.previous_name, &catalog);
                            if conflict { ui.colored_label(ui.visuals().warn_fg_color, app.tr("An RF assignment with this name already exists. Choose another name.")); }
                            if ui.add_enabled(live_connected && !conflict && !app.rf.pending && (app.rf.assignment_dirty() || app.rf.previous_name.is_empty()) && !app.rf.draft["name"].as_str().unwrap_or_default().trim().is_empty() && valid_signal(&json!({"code":app.rf.draft["match"]["rf_code"],"bits":app.rf.draft["match"]["rf_bits"],"protocol":app.rf.draft["match"]["rf_protocol"]})) && !array(&app.rf.draft["actions"]).is_empty() && array(&app.rf.draft["actions"]).iter().all(|action|action_ready(action,&catalog)),egui::Button::new(format!("{} {}",crate::ui::icons::FLOPPY_DISK,app.tr("Save assignment")))).clicked() {
                                request(app,"binding.put",json!({"binding":app.rf.draft,"previous_name":app.rf.previous_name,"allow_keyboard":app.rf.allow_keyboard}));
                            }
                            if !app.rf.previous_name.is_empty() && ui.add_enabled(live_connected && !app.rf.pending,egui::Button::new(format!("{} {}",crate::ui::icons::TRASH,app.tr("Remove")))).clicked() { app.rf.propose(RfIntent::Remove { operation:"binding.remove",params:json!({"name":app.rf.previous_name}) }); }
                        });
                        draw_saved_assignments(app, ui, &catalog, live_connected);
                    },
                    1=> {
                        ui.horizontal(|ui| {
                            let learning=catalog["learning"]["active"].as_bool().unwrap_or(false);
                            if learning { ui.colored_label(ui.visuals().selection.bg_fill,app.tr("Learning…")); if ui.add_enabled(live_connected,egui::Button::new(app.tr("Stop learning"))).clicked() { request(app,"learn.cancel",json!({})); } }
                            else if ui.add_enabled(catalog["connected"]==true && live_connected && !app.rf.pending,egui::Button::new(format!("{} {}",crate::ui::icons::PLUS,app.tr("Learn buttons")))).clicked() { request(app,"learn.start",json!({})); }
                        });
                        let mut entries=known_signals(&catalog);
                        entries.sort_by_key(|entry|std::cmp::Reverse(entry["last_seen"].as_str().and_then(|time|time.parse::<jiff::Timestamp>().ok())));
                        for entry in entries {
                            ui.push_id(signal_key(&entry),|ui| { egui::Frame::group(ui.style()).show(ui,|ui| {
                                ui.horizontal_wrapped(|ui| {
                                    draw_reception(&app.rf,ui,&entry,live_connected,app.language);
                                    let learned=entry["id"].as_u64().is_some();
                                    ui.label(egui::RichText::new(entry["name"].as_str().filter(|s|!s.is_empty()).unwrap_or("RF button")).color(if learned {ui.visuals().text_color()} else {ui.visuals().weak_text_color()}));
                                    ui.label(egui::RichText::new(entry["code_display"].as_str().unwrap_or_default()).monospace().color(if learned {ui.visuals().text_color()} else {ui.visuals().weak_text_color()}));
                                    if learned && rf_icon_button(ui,crate::ui::icons::CPU,&app.tr("Edit board action"),!app.rf.pending).clicked() {app.rf.propose(RfIntent::Board(edit_board_map(&entry)));}
                                    if !learned && rf_icon_button(ui,crate::ui::icons::PLUS,&app.tr("Add observed button"),live_connected && catalog["connected"]==true && catalog["learning"]["active"]!=true && !app.rf.pending && array(&catalog["receptions"]).iter().any(|r|signal_key(r)==signal_key(&entry))).clicked() {request(app,"observed.add",json!({"code":entry["code"],"bits":entry["bits"],"protocol":entry["protocol"]}));}
                                    if rf_icon_button(ui,crate::ui::icons::PENCIL_SIMPLE,&app.tr("Edit assignment"),!app.rf.pending).clicked() {
                                        let existing=array(&catalog["bindings"]).into_iter().find(|binding| binding["match"]["rf_code"]==entry["code"] && binding["match"]["rf_bits"]==entry["bits"] && binding["match"]["rf_protocol"]==entry["protocol"] && binding["match"]["gesture"]=="down");
                                        let previous_name=existing.as_ref().and_then(|binding|binding["name"].as_str()).unwrap_or_default().into();
                                        let draft=existing.unwrap_or_else(|| { let mut binding=new_binding(); binding["name"]=json!(unique_binding_name(&format!("RF {}",entry["code_display"].as_str().unwrap_or_default()),&catalog)); binding["match"]["rf_code"]=entry["code"].clone(); binding["match"]["rf_bits"]=entry["bits"].clone(); binding["match"]["rf_protocol"]=entry["protocol"].clone(); binding });
                                        app.rf.propose(RfIntent::Assignment { draft, previous_name, control: None });
                                    }
                                    if rf_icon_button(ui,crate::ui::icons::PAPER_PLANE_TILT,&app.tr("Transmit"),live_connected && catalog["connected"]==true && !app.rf.pending).clicked() { request(app,"transmit",json!({"code":entry["code"],"bits":entry["bits"],"protocol":entry["protocol"],"pulse_us":entry["pulse_us"].as_u64().unwrap_or(0),"repeats":1})); }
                                    if learned { ui.menu_button(crate::ui::icons::DOTS_THREE,|ui| {
                                        let can_mutate=live_connected && catalog["connected"]==true && !app.rf.pending && catalog["learning"]["active"]!=true;
                                        if ui.add_enabled(can_mutate && entry["action_kind"].as_u64().unwrap_or(0)>0,egui::Button::new(app.tr("Unassign board action"))).clicked() { app.rf.propose(RfIntent::Remove {operation:"map",params:json!({"id":entry["id"],"action":"none"})}); ui.close(); }
                                        if ui.button(app.tr("Edit board action")).clicked() { app.rf.propose(RfIntent::Board(edit_board_map(&entry))); ui.close(); }
                                        if ui.add_enabled(can_mutate,egui::Button::new(app.tr("Remove learned button"))).clicked() { app.rf.propose(RfIntent::Remove {operation:"remove",params:json!({"id":entry["id"]})}); ui.close(); }
                                    }); }
                                });
                                if entry["id"].as_u64().is_some() { ui.label(format!("On board · {}",board_assignment(&entry))); }
                                for binding in array(&catalog["bindings"]).iter().filter(|binding|signal_matches(binding,&entry)) {
                                    let enabled=if binding["enabled"]==false {" · disabled"} else {""};
                                    ui.label(format!("{}{enabled}",binding["match"]["gesture"].as_str().unwrap_or_default()));
                                    for action in array(&binding["actions"]) {ui.label(action_summary(&action,&catalog));}
                                }
                                egui::CollapsingHeader::new(app.tr("Technical details")).show(ui,|ui| {
                                    ui.label(format!("{} · {}",app.tr("Last seen"),seen_label(&entry)));
                                    ui.label(egui::RichText::new(format!("{} bits · Protocol {}",entry["bits"],entry["protocol"])).small().weak());
                                    if let Some(pulse)=entry["pulse_us"].as_u64() {ui.label(format!("{pulse} µs"));}
                                    for binding in array(&catalog["bindings"]).iter().filter(|binding|signal_matches(binding,&entry)) {ui.label(binding["name"].as_str().unwrap_or_default());ui.label(execution_label(binding));}
                                });
                            }); });
                        }
                        if app.rf.board_map["id"].as_u64().is_some() {
                        ui.separator(); ui.strong(app.tr("On-board assignment"));
                        egui::Grid::new("rf_board_map").num_columns(2).show(ui,|ui| {
                            field_label(ui,"Learned button");
                            let caption=array(&catalog["entries"]).iter().find(|entry|entry["id"]==app.rf.board_map["id"]).map(|entry|format!("RF {} · {}",entry["id"],entry["code_display"].as_str().unwrap_or_default())).unwrap_or_else(||"Choose…".into());
                            crate::ui::dropdown::ComboBox::from_id_salt("board_learned_button").width(field_width(ui)).selected_text(caption).truncate().show_ui(ui,|ui| {
                                for entry in array(&catalog["entries"]) { if ui.dropdown_choice(app.rf.board_map["id"]==entry["id"],format!("RF {} · {}",entry["id"],entry["code_display"].as_str().unwrap_or_default())).clicked() {app.rf.propose(RfIntent::Board(edit_board_map(&entry)));} }
                            }); ui.end_row();
                            let previous=app.rf.board_map["action"].clone();
                            choices(ui,"Action",&mut app.rf.board_map,"action",&options(&catalog["board_mapping_actions"]));
                            if previous!=app.rf.board_map["action"] {app.rf.board_map.as_object_mut().unwrap().remove("target");app.rf.board_map.as_object_mut().unwrap().remove("behavior");}
                            if let Some(option)=array(&catalog["board_mapping_options"]).iter().find(|option|option["action"]==app.rf.board_map["action"]) {
                                if !array(&option["targets"]).is_empty() { choices(ui,"Target",&mut app.rf.board_map,"target",&target_options(&catalog,option)); }
                                if !array(&option["behaviors"]).is_empty() { choices(ui,"Behavior",&mut app.rf.board_map,"behavior",&behavior_options(option)); }
                            }
                        });
                        if ui.add_enabled(app.rf.board_dirty() && !app.rf.pending && catalog["connected"]==true && catalog["learning"]["active"]!=true && live_connected && board_map_valid(&app.rf.board_map,&catalog),egui::Button::new(app.tr("Save on board"))).clicked() {
                            let mut map=app.rf.board_map.clone(); if map["action"]=="none" { map.as_object_mut().unwrap().remove("target"); }
                            if map["action"]=="none" || map["action"]=="menu" { map.as_object_mut().unwrap().remove("behavior"); }
                            request(app,"map",map);
                        }
                        if ui.button(app.tr("Cancel")).clicked() {app.rf.propose(RfIntent::Board(json!({"id":null,"action":""})));}
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
                            field_label(ui,"Code (hex or decimal)"); ui.add(crate::ui::dialog::singleline_text_edit(ui,&mut app.rf.code).desired_width(field_width(ui))); ui.end_row();
                            field_label(ui,"Bits"); ui.add(egui::DragValue::new(&mut app.rf.bits).range(1..=32)); ui.end_row();
                            field_label(ui,"Protocol"); ui.add(egui::DragValue::new(&mut app.rf.protocol).range(1..=12)); ui.end_row();
                            field_label(ui,"Pulse (µs)"); ui.add(egui::DragValue::new(&mut app.rf.pulse_us)).on_hover_text("0 = protocol default"); ui.end_row();
                            field_label(ui,"Repeats"); ui.add(egui::DragValue::new(&mut app.rf.repeats).range(1..=20)); ui.end_row();
                        });
                        let code=app.rf.code.trim(); let parsed=code.strip_prefix("0x").or_else(||code.strip_prefix("0X")).map(|s|u32::from_str_radix(s,16)).unwrap_or_else(||code.parse::<u32>()).ok().filter(|v|*v!=0);
                        if ui.add_enabled(parsed.is_some() && !app.rf.pending && catalog["connected"]==true && live_connected,egui::Button::new(format!("{} Transmit",crate::ui::icons::PAPER_PLANE_TILT))).clicked() { request(app,"transmit",json!({"code":parsed,"bits":app.rf.bits,"protocol":app.rf.protocol,"pulse_us":app.rf.pulse_us,"repeats":app.rf.repeats})); }
                    },
                    3=> {
                        for event in array(&catalog["activity"]) { ui.label(event["text"].as_str().unwrap_or_default()); ui.label(egui::RichText::new(event["time"].as_str().unwrap_or_default()).small().weak()); ui.separator(); }
                    },
                    _=> {
                        if !app.rf.preferences_draft.is_object() { return; }
                        let labels=[app.tr("Timed learning"),app.tr("Timeout"),app.tr("Continuously learn buttons"),app.tr("Save settings")];
                        let draft=&mut app.rf.preferences_draft;
                        let mut timed=draft["timed"].as_bool().unwrap_or(false);
                        let mut seconds=draft["timeout_seconds"].as_u64().unwrap_or(60);
                        let mut continuous=draft["continuous"].as_bool().unwrap_or(false);
                        ui.add_enabled_ui(!app.rf.pending,|ui| {
                            ui.checkbox(&mut timed,&labels[0]);
                            ui.horizontal(|ui| {ui.label(&labels[1]);ui.add(egui::DragValue::new(&mut seconds).range(1..=120).suffix(" s"));});
                            ui.checkbox(&mut continuous,&labels[2]);
                        });
                        *draft=json!({"timed":timed,"timeout_seconds":seconds,"continuous":continuous});
                        if ui.add_enabled(!app.rf.pending && live_connected && catalog["preferences"].is_object() && *draft!=catalog["preferences"],egui::Button::new(&labels[3])).clicked() {
                            let value=draft.clone();request(app,"preferences.set",value);
                        }
                    }
                } });
            });
        });
    if app.rf.raise_on_open {
        if let Some(window) = window.as_ref() { ui.ctx().move_to_top(window.response.layer_id); }
        app.rf.raise_on_open = false;
    }
    if !open { app.rf.propose(RfIntent::Close); }
    if app.rf.confirmation.is_none() && !app.rf.pending && !egui::Popup::is_any_open(ui.ctx())
        && window.as_ref().is_some_and(|window| ui.ctx().memory(|memory|memory.areas().top_layer_id(egui::Order::Foreground))==Some(window.response.layer_id))
        && ui.ctx().input_mut(|input|input.consume_key(egui::Modifiers::NONE,egui::Key::Escape)) {
        app.rf.propose(RfIntent::Close);
    }
    if let Some(intent)=app.rf.confirmation.clone() {
        let destructive=matches!(intent,RfIntent::Remove {..});
        let mut accept=false;let mut cancel=false;
        let response=crate::ui::dialog::confirmation_modal(ui.ctx(),egui::Id::new("rf-confirm-change"),
            &app.tr(if destructive {"Remove RF assignment"} else {"Unsaved RF changes"}),crate::ui::icons::WARNING,
            &app.tr(if destructive {"Remove this RF assignment?"} else {"Discard unsaved RF changes?"}),&app.tr("Close"),|ui| {
                if ui.button(app.tr("Cancel")).clicked() {cancel=true;}
                if ui.add_enabled(!app.rf.pending && (!destructive || live_connected),egui::Button::new(app.tr(if destructive {"Remove"} else {"Discard"}))).clicked() {accept=true;}
                accept || cancel
            });
        if response.dismissed || cancel {app.rf.confirmation=None;}
        if accept {
            app.rf.confirmation=None;
            match intent {RfIntent::Remove {operation,params}=>request(app,operation,params),other=>app.rf.apply_intent(other)}
        }
    }
}

fn actions_len_button(ui: &mut egui::Ui) -> egui::Response {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        rf_icon_button(ui,crate::ui::icons::TRASH,&rf_tr(ui,"Remove action"),true)
    })
    .inner
}

#[cfg(test)]
mod tests {
    #[test]
    fn native_rf_tabs_draw_without_mutations_and_escape_preserves_dirty_edits() {
        let _guard=crate::app::tests::lock_app_tests();
        let mut app=PealayerApp::default();app.engine_handle=crate::four_d::engine::memory_only_test_engine();
        app.language=crate::config::AppLanguage::English;app.rf.open=true;
        let catalog=json!({"connected":true,"host_instance_id":"fixture","connection_generation":1,"entries_sampled":true,
            "entries":[{"id":0,"code":42,"code_display":"0x0000002A","bits":24,"protocol":1,"board_assignment":{"action":"none","label":"Unassigned"}}],
            "receptions":[{"code":99,"bits":24,"protocol":1,"received_count":8}],
            "preferences":{"timed":false,"timeout_seconds":60,"continuous":false},
            "gestures":["down","tap","hold","repeat","hold-end"],"action_types":["control"],
            "peripherals":{"controls":[{"key":"pwm.1","name":"Loostr / Chandelier with a long renamable title","actions":[{"id":"pwm.1.on","name":"On"}]}]},
            "bindings":[{"name":"Saved Loostr","enabled":true,"match":{"rf_code":42,"rf_bits":24,"rf_protocol":1,"gesture":"tap"},"actions":[{"type":"control","action_id":"pwm.1.on"}]}],
            "board_mapping_options":[{"action":"none","targets":[],"behaviors":[]}],"board_mapping_actions":["none"]});
        app.rf.apply_catalog(catalog.clone());
        app.rf.apply_intent(RfIntent::Board(edit_board_map(&catalog["entries"][0])));
        for width in [360.0,480.0,980.0] {
            let context=egui::Context::default();
            for tab in 0..5 {
                app.rf.tab=tab;
                for time in [0.0,0.1,0.2] {
                    let mut output=context.run_ui(egui::RawInput {screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(width,780.0))),time:Some(time),..Default::default()},|ui|draw(&mut app,ui));
                    assert_eq!(app.rf.catalog,catalog,"drawing a tab must not learn or alter assignments/preferences");
                    assert!(!app.rf.pending,"no user event means no RPC");output.textures_delta.clear();
                }
            }
        }
        let context=egui::Context::default();
        app.rf.tab=0;app.rf.draft["name"]=json!("Unsaved test");
        for time in [0.0,0.1] {let mut output=context.run_ui(egui::RawInput {time:Some(time),..Default::default()},|ui|draw(&mut app,ui));output.textures_delta.clear();}
        let escape=egui::Event::Key {key:egui::Key::Escape,physical_key:None,pressed:true,repeat:false,modifiers:egui::Modifiers::NONE};
        let mut output=context.run_ui(egui::RawInput {time:Some(0.2),events:vec![escape],..Default::default()},|ui|draw(&mut app,ui));output.textures_delta.clear();
        assert!(app.rf.open);assert!(matches!(app.rf.confirmation,Some(RfIntent::Close)));assert_eq!(app.rf.draft["name"],"Unsaved test");
    }
    #[test]
    fn unsaved_close_and_retarget_require_confirmation_without_resetting_receptions() {
        let mut state=super::RfState::default();
        state.open=true;state.begin_reception_session();
        state.draft["name"]=serde_json::json!("Unsaved Lantern");
        let original=state.draft.clone();
        state.propose(super::RfIntent::Close);
        assert!(state.open && state.session_open);
        assert!(state.confirmation.is_some());
        state.confirmation=None; // Cancel / × / Escape in confirmation.
        assert_eq!(state.draft,original);
        state.begin_control_assignment("pwm.2","Loostr");
        assert_eq!(state.draft,original,"opening another channel must not replace unsaved edits");
        let intent=state.confirmation.take().unwrap();state.apply_intent(intent);
        assert_eq!(state.draft["name"],"Loostr");
        assert_eq!(state.assignment_control.as_deref(),Some("pwm.2"));
        assert!(!state.assignment_dirty());
        state.draft["cooldown_ms"]=serde_json::json!(400);
        state.propose(super::RfIntent::Close);
        let intent=state.confirmation.take().unwrap();state.apply_intent(intent);
        assert!(!state.open && !state.session_open);
        assert!(!state.dirty());
    }

    #[test]
    fn failed_save_preserves_draft_and_pending_retarget_is_ignored() {
        let mut state=super::RfState::default();
        state.begin_control_assignment("pwm.0","Window");
        state.draft["match"]["rf_code"]=serde_json::json!(42);
        let draft=state.draft.clone();
        state.pending=true;state.pending_name="Window".into();state.pending_draft=Some(draft.clone());
        state.propose(super::RfIntent::Assignment {draft:super::new_binding(),previous_name:String::new(),control:None});
        assert_eq!(state.draft,draft);
        assert!(state.confirmation.is_none());
        state.complete("rf-binding.put",Err("Not saved".into()));
        assert_eq!(state.draft,draft);assert!(state.assignment_dirty());
        state.pending_name="Window".into();state.pending_draft=Some(draft.clone());
        state.complete("rf-binding.put",Ok(serde_json::json!([draft])));
        assert!(!state.assignment_dirty());assert_eq!(state.previous_name,"Window");
    }

    #[test]
    fn catalog_refresh_cannot_carry_entries_or_reception_dates_across_owner_or_generation() {
        for replacement in [serde_json::json!({"host_instance_id":"b","connection_generation":1}),serde_json::json!({"host_instance_id":"a","connection_generation":2})] {
            let mut state=super::RfState::default();
            state.apply_catalog(serde_json::json!({"host_instance_id":"a","hostname":"same-host","connection_generation":1,"connected":true,"entries_sampled":true,
                "entries":[{"id":0,"code":42,"bits":24,"protocol":1,"last_seen":"2026-10-14T10:00:00Z"}]}));
            let mut next=replacement;
            next["hostname"]=serde_json::json!("same-host");next["connected"]=serde_json::json!(true);
            state.apply_catalog(next.clone());
            assert!(super::array(&state.catalog["entries"]).is_empty());
            next["entries_sampled"]=serde_json::json!(true);next["entries"]=serde_json::json!([{"id":0,"code":42,"bits":24,"protocol":1}]);
            state.apply_catalog(next);
            assert!(state.catalog["entries"][0]["last_seen"].is_null());
        }
    }

    #[test]
    fn known_codes_require_complete_waveforms_and_board_labels_are_authoritative() {
        let catalog=serde_json::json!({"entries":[{"code":0,"bits":24,"protocol":1}],"bindings":[
            {"match":{"kind":"host.event"}}, {"match":{"rf_code":9,"rf_bits":0,"rf_protocol":1}},
            {"match":{"rf_code":42,"rf_bits":24,"rf_protocol":1}}]});
        assert_eq!(super::known_signals(&catalog).len(),1);
        let labels=super::behavior_options(&serde_json::json!({"behaviors":["toggle","roll"],"behavior_labels":{"toggle":"Restore last brightness","roll":"Roll brightness"}}));
        assert_eq!(labels[0],("toggle".into(),"Restore last brightness".into()));
    }

    #[test]
    fn shared_learning_preferences_only_become_clean_after_ack() {
        let mut state=super::RfState::default();
        state.apply_catalog(serde_json::json!({"preferences":{"timed":false,"timeout_seconds":60,"continuous":false}}));
        assert!(!state.dirty());
        state.preferences_draft["continuous"]=serde_json::json!(true);
        let draft=state.preferences_draft.clone();
        state.apply_catalog(state.catalog.clone());assert_eq!(state.preferences_draft,draft);
        state.complete("rf-preferences.set",Err("Save failed".into()));assert!(state.dirty());
        state.complete("rf-preferences.set",Ok(draft));assert!(!state.dirty());
    }

    #[test]
    fn rf_icon_actions_are_exact_squares_across_density_fonts_and_scales() {
        for density in crate::config::UiControlDensity::ALL {
            for font in [12.0,20.0,24.0] {
                for scale in [1.0,1.25,2.0] {
                    let context=egui::Context::default();context.set_pixels_per_point(scale);
                    context.all_styles_mut(|style| {crate::ui::control_density::configure(style,density);style.text_styles.insert(egui::TextStyle::Button,egui::FontId::proportional(font));});
                    let mut previous=None;
                    for enabled in [true,false,true] {
                        let mut rect=egui::Rect::NOTHING;
                        let mut output=context.run_ui(egui::RawInput::default(),|ui| {rect=super::rf_icon_button(ui,crate::ui::icons::PENCIL_SIMPLE,"Edit",enabled).rect;});
                        assert!((rect.width()-rect.height()).abs()<0.01);
                        if let Some(previous)=previous {assert_eq!(rect,previous);}
                        previous=Some(rect);output.textures_delta.clear();
                    }
                }
            }
        }
    }
    #[test]
    fn reception_counter_resets_on_open_not_refresh_and_includes_unlearned_signals() {
        let mut state=super::RfState::default();
        let time=jiff::Timestamp::now().to_string();
        let receipt=serde_json::json!({"code":42,"bits":24,"protocol":1,"received_count":100,"last_seen":time});
        state.apply_catalog(serde_json::json!({"receptions":[receipt.clone()],"entries":[]}));
        state.begin_reception_session();
        assert_eq!(super::reception_status(&state,&receipt,true).1,0);
        let mut updated=state.catalog.clone();updated["receptions"][0]["received_count"]=serde_json::json!(109);
        state.apply_catalog(updated.clone());
        assert_eq!(super::reception_status(&state,&receipt,true).1,9);
        state.apply_catalog(updated);
        assert_eq!(super::reception_status(&state,&receipt,true).1,9);
        assert_eq!(super::known_signals(&state.catalog).len(),1);
        assert!(super::known_signals(&state.catalog)[0]["id"].is_null());
        state.session_open=false;state.begin_reception_session();
        assert_eq!(super::reception_status(&state,&receipt,true).1,0);
        assert!(!super::reception_status(&state,&receipt,false).0);
    }
    #[test]
    fn rf_error_headline_uses_metadata_not_human_prose() {
        assert_eq!(super::error_caption(&serde_json::json!({"kind":"timeout"})), Some("PCController request timed out"));
        assert_eq!(super::error_caption(&serde_json::json!({"transport_failed":true})), Some("PCController connection is unavailable"));
        assert_eq!(super::error_caption(&serde_json::json!({"kind":"authority_conflict","message":"timed out (os error 10060)"})), None);
        assert_eq!(super::error_caption(&serde_json::json!("timed out")), None);
    }
    #[test]
    fn saved_rf_rules_group_by_complete_waveform_and_authoritative_channel_without_mutation() {
        let rules = ["tap", "hold", "repeat", "hold-end"].map(|gesture| serde_json::json!({
            "name":gesture,"match":{"rf_code":42,"rf_bits":24,"rf_protocol":1,"gesture":gesture},
            "actions":[{"type":"board","command":"pwm roll 1"}]}));
        let catalog = serde_json::json!({"bindings":rules});
        let before = catalog.clone();
        let groups = super::grouped_bindings(&catalog, Some("pwm.1"));
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 4, "four internal rules are one physical button");
        assert!(super::grouped_bindings(&catalog, Some("pwm.0")).is_empty());
        assert_eq!(catalog, before);
        assert!(super::pwm_board_action(&serde_json::json!({"type":"board","command":"pwm roll 16"})).is_none());
        assert!(super::pwm_board_action(&serde_json::json!({"type":"board","command":"pwm roll 1 extra"})).is_none());
        let mut different = catalog.clone();
        different["bindings"][1]["match"]["rf_bits"] = serde_json::json!(16);
        assert_eq!(super::grouped_bindings(&different, None).len(), 2);
    }
    use super::*;
    use serde_json::json;

    #[test]
    fn fresh_rf_assignments_have_unique_bounded_names_and_edits_are_explicit() {
        let mut catalog = json!({"bindings":[]});
        for _ in 0..4 {
            let name = unique_binding_name("Light", &catalog);
            assert!(!binding_name_conflicts(&name, "", &catalog));
            catalog["bindings"].as_array_mut().unwrap().push(json!({"name":name}));
        }
        assert_eq!(catalog["bindings"][3]["name"], "Light 4");
        assert!(binding_name_conflicts("light", "", &catalog));
        assert!(!binding_name_conflicts("light", "Light", &catalog));
        assert!(binding_name_conflicts("Light 2", "Light", &catalog));
        let preferred = "چراغ".repeat(30);
        let name = unique_binding_name(&preferred, &catalog);
        assert!(name.len() <= 64);
        catalog["bindings"].as_array_mut().unwrap().push(json!({"name":name}));
        let next = unique_binding_name(&preferred, &catalog);
        assert!(next.len() <= 64);
        assert!(!binding_name_conflicts(&next, "", &catalog));
    }

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
