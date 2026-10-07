use crate::app::PealayerApp;
use eframe::egui;
use serde_json::{Value, json};
use std::time::Duration;

pub const OPERATIONS: &[&str] = &["catalog", "learn.start", "learn.status", "learn.cancel", "map", "remove", "clear", "transmit", "binding.put", "binding.remove"];

pub struct RfState {
    pub open: bool,
    pub pending: bool,
    pub error: String,
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
}

fn new_binding() -> Value {
    json!({"name":"", "enabled":true,"cooldown_ms":250,
        "match":{"kind":"rf.gesture","source":"rf","rf_code":0,"rf_bits":24,"rf_protocol":1,"gesture":"down"},
        "actions":[{"type":"app","app_target":"pealayer","app_kind":"pealayer.toggle","app_value":""}]})
}

impl Default for RfState {
    fn default() -> Self {
        Self { open:false, pending:false, error:String::new(), catalog:Value::Null,
            last_result:Value::Null, last_operation:String::new(), error_is_catalog:false,
            pending_name:String::new(), tab:0, draft:new_binding(),
            previous_name:String::new(), allow_keyboard:false, code:String::new(), bits:24,
            protocol:1, pulse_us:0, repeats:1, board_map:json!({"id":0,"action":"none"}) }
    }
}

impl RfState {
    pub fn snapshot(&self) -> Value {
        json!({"catalog":self.catalog,"pending":self.pending,"error":self.error,"last_result":self.last_result,"last_operation":self.last_operation})
    }
    pub fn apply_catalog(&mut self, mut value: Value) {
        if value["connected"].as_bool() == Some(true) && value["entries_sampled"].as_bool() != Some(true) {
            value["entries"] = self.catalog["entries"].clone();
            value["entries_sampled"] = self.catalog["entries_sampled"].clone();
        }
        self.catalog = value;
    }
    pub fn complete(&mut self, operation: &str, result: Result<Value, String>) -> bool {
        self.pending = false;
        match result {
            Ok(value) => {
                if operation != "rf-catalog" || self.error_is_catalog { self.error.clear(); }
                if operation == "rf-catalog" {
                    let learning_finished = self.catalog["learning"]["active"] == true && value["learning"]["active"] != true;
                    self.apply_catalog(value);
                    return learning_finished;
                }
                if operation == "rf-binding.put" && self.draft["name"].as_str() == Some(self.pending_name.as_str()) {
                    self.previous_name = self.pending_name.clone();
                }
                if operation == "rf-binding.remove" && self.previous_name == self.pending_name { self.previous_name.clear(); }
                self.last_operation = operation.into();
                self.last_result = value;
                true
            }
            Err(error) => { self.error = error; self.error_is_catalog = operation == "rf-catalog"; false }
        }
    }
}

impl PealayerApp {
    pub(crate) fn request_rf(&mut self, operation: &str, params: Value) -> Result<(), String> {
        if !OPERATIONS.contains(&operation) { return Err("Unknown RF operation".into()); }
        if self.rf.pending {
            // Desktop and web readers share one operation queue. Polls must not
            // overwrite a mutation's result or report a false failure.
            if operation == "catalog" { return Ok(()); }
            return Err("An RF operation is still pending".into());
        }
        let pending_name = if operation == "binding.put" { params["binding"]["name"].as_str() } else { params["name"].as_str() }.unwrap_or_default().to_owned();
        self.engine_handle.request_controller_call(&format!("rf-{operation}"), &format!("controller.rf.{operation}"), params)?;
        self.rf.pending_name = pending_name;
        if operation != "catalog" { self.rf.error.clear(); self.rf.error_is_catalog = false; }
        self.rf.pending = true;
        Ok(())
    }
}

fn request(app: &mut PealayerApp, operation: &str, params: Value) {
    if let Err(error) = app.request_rf(operation, params) { app.rf.error = error; }
}

fn text(ui: &mut egui::Ui, label: &str, value: &mut Value, field: &str) {
    ui.label(label);
    let mut current = value[field].as_str().unwrap_or_default().to_string();
    if ui.add(egui::TextEdit::singleline(&mut current).desired_width(ui.available_width())).changed() { value[field] = json!(current); }
    ui.end_row();
}

fn choices(ui: &mut egui::Ui, label: &str, value: &mut Value, field: &str, options: &[(String,String)]) {
    ui.label(label);
    let mut selected = value[field].as_str().unwrap_or_default().to_string();
    let caption = options.iter().find(|(id,_)| *id == selected).map(|(_,name)|name.as_str()).unwrap_or(&selected);
    egui::ComboBox::from_id_salt((ui.id(),field)).selected_text(caption).width(ui.available_width().min(360.0)).show_ui(ui, |ui| {
        for (id,name) in options { ui.selectable_value(&mut selected,id.clone(),name); }
    });
    value[field] = json!(selected);
    ui.end_row();
}

fn array(value: &Value) -> Vec<Value> { value.as_array().cloned().unwrap_or_default() }
fn options(value: &Value) -> Vec<(String,String)> {
    array(value).iter().filter_map(Value::as_str).map(|v|(v.into(),v.replace(['.','-','_']," "))).collect()
}
fn number(ui: &mut egui::Ui, label:&str, value:&mut Value, field:&str, min:u64,max:u64) {
    ui.label(label); let mut n=value[field].as_u64().unwrap_or(min);
    if ui.add(egui::DragValue::new(&mut n).range(min..=max)).changed() { value[field]=json!(n); }
    ui.end_row();
}

fn action_editor(ui: &mut egui::Ui, action: &mut Value, catalog:&Value) {
    egui::Grid::new((ui.id(),"action_fields")).num_columns(2).spacing([16.0,6.0]).show(ui, |ui| {
        choices(ui,"Action",action,"type",&options(&catalog["action_types"]));
        match action["type"].as_str().unwrap_or_default() {
            "app" => {
                let applications=array(&catalog["applications"]);
                let mut targets=Vec::new();
                for app in &applications {
                    if let (Some(surface),Some(id))=(app["surface"].as_str(),app["id"].as_str()) {
                        if !targets.iter().any(|(key,_)|key==surface) { targets.push((surface.into(),format!("{surface} (all)"))); }
                        targets.push((id.into(),id.into()));
                    }
                }
                choices(ui,"Application",action,"app_target",&targets);
                let target=action["app_target"].as_str().unwrap_or_default();
                let mut kinds=Vec::new();
                for app in applications {
                    if app["id"].as_str()==Some(target) || app["surface"].as_str()==Some(target) {
                        for key in app["values"]["app_actions"].as_str().unwrap_or_default().split(',').filter(|s|!s.is_empty()) {
                            if !kinds.iter().any(|(id,_)|id==key) { kinds.push((key.into(),key.replace('.'," "))); }
                        }
                    }
                }
                choices(ui,"Command",action,"app_kind",&kinds);
                text(ui,"Value",action,"app_value");
            },
            "control" => {
                let mut actions=Vec::new();
                for control in array(&catalog["peripherals"]["controls"]) {
                    for a in array(&control["actions"]) {
                        if let Some(id)=a["id"].as_str() { actions.push((id.into(),format!("{} · {}", control["name"].as_str().unwrap_or("Control"),a["label"].as_str().or(a["name"].as_str()).unwrap_or(id)))); }
                    }
                }
                choices(ui,"Control",action,"action_id",&actions);
            },
            "effect" | "macro" => {
                let effects=array(&catalog["effects"]).iter().filter_map(|e|e["name"].as_str().map(|name|(e["reference"].as_str().unwrap_or(name).into(),name.into()))).collect::<Vec<_>>();
                choices(ui,"Effect",action,"macro",&effects);
            },
            "board" => text(ui,"Controller command",action,"command"),
            "virtual-key" => { text(ui,"Controller host key or shortcut",action,"virtual_key"); number(ui,"Hold (ms)",action,"hold_ms",10,1000); },
            "host" | "script" => {
                let field=if action["type"]=="host" {"executable"} else {"script"};
                text(ui,"Program",action,field);
                ui.label("Arguments (one per line)");
                let mut args=array(&action["args"]).iter().filter_map(Value::as_str).collect::<Vec<_>>().join("\n");
                if ui.add(egui::TextEdit::multiline(&mut args).desired_rows(2)).changed() { action["args"]=json!(args.lines().collect::<Vec<_>>()); } ui.end_row();
                ui.label("Launch"); let mut detached=action["detached"].as_bool().unwrap_or(false); if ui.checkbox(&mut detached,"Launch independently").changed() { action["detached"]=json!(detached); } ui.end_row();
            },
            "emit" => text(ui,"Event",action,"event"),
            "rf" => {
                if !action["rf"].is_object() { action["rf"]=json!({"code":0,"bits":24,"protocol":1,"pulse_us":0,"repeats":1}); }
                for (label,field,min,max) in [("Code","code",1,u32::MAX as u64),("Bits","bits",1,32),("Protocol","protocol",1,12),("Pulse (µs)","pulse_us",0,65535),("Repeats","repeats",1,20)] { number(ui,label,&mut action["rf"],field,min,max); }
            },
            _=>{},
        }
    });
}

pub fn draw(app:&mut PealayerApp, ui:&mut egui::Ui) {
    if !app.rf.open { return; }
    ui.ctx().request_repaint_after(Duration::from_millis(250));
    let geometry=crate::ui::dialog::bounded_geometry(ui.ctx().content_rect(),24.0,egui::vec2(820.0,610.0),egui::vec2(480.0,380.0),egui::vec2(980.0,760.0));
    let mut open=true;
    let catalog=app.rf.catalog.clone();
    egui::Window::new(format!("{} RF controls",crate::ui::icons::RADIO)).id(egui::Id::new("rf_manager"))
        .open(&mut open).default_rect(geometry.default_rect).min_size(geometry.min_size).max_size(geometry.max_size)
        .constrain_to(geometry.bounds).resizable(true).collapsible(false).order(egui::Order::Foreground)
        .frame(crate::ui::dialog::opaque_window_frame(ui)).show(ui.ctx(),|ui| {
            ui.horizontal(|ui| {
                for (i,caption) in ["Assignments","Remotes","Transmit","Activity"].iter().enumerate() { ui.selectable_value(&mut app.rf.tab,i,*caption); }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui| {
                    if ui.add_enabled(!app.rf.pending,egui::Button::new(crate::ui::icons::ARROW_CLOCKWISE)).on_hover_text("Refresh learned buttons").clicked() { request(app,"catalog",json!({"read_board":true})); }
                    if app.rf.pending { ui.spinner(); }
                });
            });
            ui.separator();
            if !app.rf.error.is_empty() { ui.colored_label(ui.visuals().error_fg_color,&app.rf.error); }
            if let Some(error)=catalog["board_error"].as_str() { ui.colored_label(ui.visuals().warn_fg_color,error); }
            egui::ScrollArea::vertical().id_salt("rf_body").auto_shrink([false,false]).show(ui,|ui| {
                ui.set_min_width(ui.available_width());
                match app.rf.tab {
                    0=> {
                        ui.horizontal(|ui| {
                            if ui.button(format!("{} New assignment",crate::ui::icons::PLUS)).clicked() { app.rf.draft=new_binding(); app.rf.previous_name.clear(); }
                            if let Some(event)=array(&catalog["activity"]).iter().find(|e|e["rf_code"].as_u64().unwrap_or(0)>0) {
                                if ui.button("Use last received").clicked() {
                                    for (to,from) in [("rf_code","rf_code"),("rf_bits","rf_bits"),("rf_protocol","rf_protocol")] { app.rf.draft["match"][to]=event[from].clone(); }
                                }
                            }
                        });
                        for binding in array(&catalog["bindings"]) {
                            ui.horizontal(|ui| {
                                let mut enabled=binding["enabled"].as_bool().unwrap_or(false);
                                if ui.checkbox(&mut enabled,"").changed() { let mut updated=binding.clone(); updated["enabled"]=json!(enabled); request(app,"binding.put",json!({"binding":updated})); }
                                if ui.selectable_label(app.rf.previous_name==binding["name"].as_str().unwrap_or_default(),binding["name"].as_str().unwrap_or("Assignment")).clicked() { app.rf.draft=binding.clone(); app.rf.previous_name=binding["name"].as_str().unwrap_or_default().into(); }
                                ui.label(egui::RichText::new(format!("{} · {}",binding["match"]["rf_code"],binding["match"]["gesture"].as_str().unwrap_or_default())).weak());
                            });
                        }
                        ui.separator();
                        egui::Grid::new("rf_binding_fields").num_columns(2).spacing([16.0,6.0]).show(ui,|ui| {
                            text(ui,"Name",&mut app.rf.draft,"name");
                            for (label,field,min,max) in [("Code","rf_code",0,u32::MAX as u64),("Bits","rf_bits",1,32),("Protocol","rf_protocol",1,12)] { number(ui,label,&mut app.rf.draft["match"],field,min,max); }
                            choices(ui,"Gesture",&mut app.rf.draft["match"],"gesture",&options(&catalog["gestures"]));
                            number(ui,"Cooldown (ms)",&mut app.rf.draft,"cooldown_ms",1,3_600_000);
                        });
                        let mut remove=None;
                        if let Some(actions)=app.rf.draft["actions"].as_array_mut() {
                            for (i,action) in actions.iter_mut().enumerate() {
                                ui.push_id(i,|ui| {
                                    egui::Frame::group(ui.style()).inner_margin(10.0).show(ui,|ui| {
                                        ui.horizontal(|ui| { ui.strong(format!("Action {}",i+1)); if actions_len_button(ui).clicked() { remove=Some(i); } });
                                        action_editor(ui,action,&catalog);
                                    });
                                }); ui.add_space(6.0);
                            }
                            if let Some(i)=remove { actions.remove(i); }
                            if actions.len()<8 && ui.button(format!("{} Add action",crate::ui::icons::PLUS)).clicked() { actions.push(json!({"type":"app","app_target":"pealayer","app_kind":"pealayer.toggle"})); }
                        }
                        if array(&app.rf.draft["actions"]).iter().any(|a|a["type"]=="virtual-key") {
                            ui.checkbox(&mut app.rf.allow_keyboard,format!("Allow assigned keyboard keys on {}",catalog["hostname"].as_str().unwrap_or("controller host")));
                        }
                        if array(&catalog["entries"]).iter().any(|entry| entry["code"]==app.rf.draft["match"]["rf_code"] && entry["action_kind"].as_u64().unwrap_or(0)>0) {
                            ui.colored_label(ui.visuals().warn_fg_color,"This button also has a board action. Unassign it in Remotes to replace it.");
                        }
                        ui.separator();
                        ui.horizontal(|ui| {
                            if ui.add_enabled(!app.rf.pending,egui::Button::new(format!("{} Save assignment",crate::ui::icons::FLOPPY_DISK))).clicked() {
                                request(app,"binding.put",json!({"binding":app.rf.draft,"previous_name":app.rf.previous_name,"allow_keyboard":app.rf.allow_keyboard}));
                            }
                            if !app.rf.previous_name.is_empty() && ui.add_enabled(!app.rf.pending,egui::Button::new(format!("{} Remove",crate::ui::icons::TRASH))).clicked() { request(app,"binding.remove",json!({"name":app.rf.previous_name})); }
                        });
                    },
                    1=> {
                        ui.horizontal(|ui| {
                            let learning=catalog["learning"]["active"].as_bool().unwrap_or(false);
                            if learning { ui.colored_label(ui.visuals().selection.bg_fill,"Learning…"); if ui.button("Stop learning").clicked() { request(app,"learn.cancel",json!({})); } }
                            else if ui.add_enabled(catalog["connected"]==true && !app.rf.pending,egui::Button::new(format!("{} Learn buttons",crate::ui::icons::PLUS))).clicked() { request(app,"learn.start",json!({"mode":"timer","timeout_ms":30000})); }
                        });
                        if array(&catalog["entries"]).is_empty() {
                            ui.label(egui::RichText::new("No learned RF buttons. Refresh to read the board or learn a new button.").weak());
                        }
                        for entry in array(&catalog["entries"]) {
                            egui::Frame::group(ui.style()).show(ui,|ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.strong(entry["name"].as_str().filter(|s|!s.is_empty()).unwrap_or("RF button"));
                                    ui.monospace(entry["code_display"].as_str().unwrap_or_default());
                                    if ui.button("Assign…").clicked() {
                                        let existing=array(&catalog["bindings"]).into_iter().find(|binding| binding["match"]["rf_code"]==entry["code"] && binding["match"]["rf_bits"]==entry["bits"] && binding["match"]["rf_protocol"]==entry["protocol"] && binding["match"]["gesture"]=="down");
                                        app.rf.previous_name=existing.as_ref().and_then(|binding|binding["name"].as_str()).unwrap_or_default().into();
                                        app.rf.draft=existing.unwrap_or_else(|| { let mut binding=new_binding(); binding["name"]=json!(format!("RF {}",entry["id"])); binding["match"]["rf_code"]=entry["code"].clone(); binding["match"]["rf_bits"]=entry["bits"].clone(); binding["match"]["rf_protocol"]=entry["protocol"].clone(); binding });
                                        app.rf.tab=0;
                                    }
                                    if ui.add_enabled(!app.rf.pending,egui::Button::new("Transmit")).clicked() { request(app,"transmit",json!({"code":entry["code"],"bits":entry["bits"],"protocol":entry["protocol"],"pulse_us":entry["pulse_us"],"repeats":1})); }
                                    ui.menu_button(crate::ui::icons::DOTS_THREE,|ui| {
                                        if ui.button("Unassign board action").clicked() { request(app,"map",json!({"id":entry["id"],"action":"none"})); ui.close(); }
                                        if ui.button("Edit board action…").clicked() { app.rf.board_map=json!({"id":entry["id"],"action":"none"}); ui.close(); }
                                        if ui.button("Remove learned button").clicked() { request(app,"remove",json!({"id":entry["id"]})); ui.close(); }
                                    });
                                });
                                ui.label(egui::RichText::new(format!("{} bits · Protocol {} · {} µs · Board action {} / {}",entry["bits"],entry["protocol"],entry["pulse_us"],entry["action_kind"],entry["action_value"])).weak());
                            });
                        }
                        ui.separator(); ui.strong("Board assignment");
                        egui::Grid::new("rf_board_map").num_columns(2).show(ui,|ui| {
                            number(ui,"Button ID",&mut app.rf.board_map,"id",0,19);
                            choices(ui,"Action",&mut app.rf.board_map,"action",&options(&catalog["board_mapping_actions"]));
                            if let Some(option)=array(&catalog["board_mapping_options"]).iter().find(|option|option["action"]==app.rf.board_map["action"]) {
                                if !array(&option["targets"]).is_empty() { choices(ui,"Target",&mut app.rf.board_map,"target",&options(&option["targets"])); }
                                if !array(&option["behaviors"]).is_empty() { choices(ui,"Behavior",&mut app.rf.board_map,"behavior",&options(&option["behaviors"])); }
                            }
                        });
                        if ui.add_enabled(!app.rf.pending && catalog["connected"]==true,egui::Button::new("Save board assignment")).clicked() {
                            let mut map=app.rf.board_map.clone(); if map["action"]=="none" { map.as_object_mut().unwrap().remove("target"); }
                            if map["action"]=="none" || map["action"]=="menu" { map.as_object_mut().unwrap().remove("behavior"); }
                            request(app,"map",map);
                        }
                    },
                    2=> {
                        egui::Grid::new("rf_tx").num_columns(2).spacing([16.0,8.0]).show(ui,|ui| {
                            ui.label("Code (hex or decimal)"); ui.text_edit_singleline(&mut app.rf.code); ui.end_row();
                            ui.label("Bits"); ui.add(egui::DragValue::new(&mut app.rf.bits).range(1..=32)); ui.end_row();
                            ui.label("Protocol"); ui.add(egui::DragValue::new(&mut app.rf.protocol).range(1..=12)); ui.end_row();
                            ui.label("Pulse (µs, 0 = protocol default)"); ui.add(egui::DragValue::new(&mut app.rf.pulse_us)); ui.end_row();
                            ui.label("Repeats"); ui.add(egui::DragValue::new(&mut app.rf.repeats).range(1..=20)); ui.end_row();
                        });
                        let code=app.rf.code.trim(); let parsed=code.strip_prefix("0x").or_else(||code.strip_prefix("0X")).map(|s|u32::from_str_radix(s,16)).unwrap_or_else(||code.parse::<u32>()).ok().filter(|v|*v!=0);
                        if ui.add_enabled(parsed.is_some() && !app.rf.pending && catalog["connected"]==true,egui::Button::new(format!("{} Transmit",crate::ui::icons::PAPER_PLANE_TILT))).clicked() { request(app,"transmit",json!({"code":parsed,"bits":app.rf.bits,"protocol":app.rf.protocol,"pulse_us":app.rf.pulse_us,"repeats":app.rf.repeats})); }
                    },
                    _=> {
                        for event in array(&catalog["activity"]) { ui.label(event["text"].as_str().unwrap_or_default()); ui.label(egui::RichText::new(event["time"].as_str().unwrap_or_default()).small().weak()); ui.separator(); }
                    }
                }
            });
        });
    app.rf.open=open;
}

fn actions_len_button(ui:&mut egui::Ui) -> egui::Response { ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui| ui.button(crate::ui::icons::TRASH).on_hover_text("Remove action")).inner }

#[cfg(test)]
mod tests {
    use super::RfState;
    use serde_json::json;

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
}
