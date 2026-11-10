//! Project metadata editor and reusable scene/timeline document actions.
use crate::{app::PealayerApp,projects::*};
use eframe::egui;
use std::{path::PathBuf,sync::{Arc,Mutex}};

#[derive(Clone)]
struct Editor {
    document:Document, baseline:Document, alternate_parts:Project, path:String, include:Components,
    error:Option<String>, notice:Option<String>, program_warning:Option<String>, candidates:Vec<PathBuf>,
    discard:bool, confirm_outputs:bool, busy:bool,
    selected:std::collections::BTreeSet<String>,
    authority: Option<uuid::Uuid>,
    scene_report:Option<serde_json::Value>,
    next_scene_poll:std::time::Instant,
    reply:Arc<Mutex<Option<Result<serde_json::Value,String>>>>,
}
fn id()->egui::Id {egui::Id::new("project-metadata-editor")}
fn local_authority() -> Option<uuid::Uuid> {
    (!crate::peer::active()).then(|| crate::peer::instance_id().parse().ok()).flatten()
}
fn reject_changed_authority(editor: &mut Editor) {
    editor.error = Some("Document authority changed. Read or capture from the new authority before applying changes.".into());
    editor.confirm_outputs = false;
}

pub(crate) fn dirty(ctx:&egui::Context)->bool {ctx.data_mut(|d|d.get_temp::<Editor>(id())).is_some_and(|e|included(&e.document,e.include)!=e.baseline)}
pub(crate) fn busy(ctx:&egui::Context)->bool {ctx.data_mut(|d|d.get_temp::<Editor>(id())).is_some_and(|e|e.busy)}
pub(crate) fn open(app:&PealayerApp,ctx:&egui::Context) {
    if ctx.data_mut(|d|d.get_temp::<Editor>(id())).is_some() {return;}
    let document=Document::Project(app.capture_project(ctx));
    let state=Editor {baseline:document.clone(),document,alternate_parts:Project::default(),path:app.project.path.as_ref().map(|p|p.to_string_lossy().into_owned()).unwrap_or_default(),include:Components::default(),error:None,notice:None,program_warning:None,candidates:vec![],discard:false,confirm_outputs:false,busy:false,selected:Default::default(),authority:local_authority(),scene_report:None,next_scene_poll:std::time::Instant::now(),reply:Arc::new(Mutex::new(None))};
    let mut state=state;
    state.scene_report=app.project.scene_operation.as_ref().map(|op|serde_json::json!(op.report()));
    ctx.data_mut(|d|d.insert_temp(id(),state));
    if crate::peer::active() {queue_remote(ctx,Request::Snapshot);}
}
pub(crate) fn open_candidates(ctx:&egui::Context,candidates:Vec<PathBuf>,error:Option<String>) {
    if let Some(mut editor)=ctx.data_mut(|d|d.get_temp::<Editor>(id())) {
        editor.candidates=candidates;editor.error=error;
        ctx.data_mut(|d|d.insert_temp(id(),editor));
        return;
    }
    let document=Document::Project(Project::default());
    ctx.data_mut(|d|d.insert_temp(id(),Editor {baseline:document.clone(),document,alternate_parts:Project::default(),path:candidates.first().map(|p|p.to_string_lossy().into_owned()).unwrap_or_default(),include:Components::default(),error,notice:None,program_warning:None,candidates,discard:false,confirm_outputs:false,busy:false,selected:Default::default(),authority:local_authority(),scene_report:None,next_scene_poll:std::time::Instant::now(),reply:Arc::new(Mutex::new(None))}));
}
fn queue_remote(ctx:&egui::Context,request:Request) {
    let Some(client)=crate::peer::client().cloned() else {return};
    let Some(mut editor)=ctx.data_mut(|d|d.get_temp::<Editor>(id())) else {return};
    let endpoint = if request.mutates_authority() {
        let observed = client.snapshot().and_then(|state| uuid::Uuid::parse_str(&state.session.instance_id).ok());
        let Some(owner) = editor.authority.filter(|owner| Some(*owner) == observed) else {
            reject_changed_authority(&mut editor);
            ctx.data_mut(|data| data.insert_temp(id(), editor));
            return;
        };
        format!("/api/peer/documents?owner_id={owner}")
    } else {
        "/api/peer/documents".into()
    };
    editor.busy=true;let reply=editor.reply.clone();ctx.data_mut(|d|d.insert_temp(id(),editor));
    let ctx=ctx.clone();
    std::thread::spawn(move||{let result=client.post(&endpoint,&serde_json::to_value(request).unwrap());if let Ok(mut r)=reply.lock(){*r=Some(result);}ctx.request_repaint();});
}
fn accept(editor:&mut Editor,result:Result<serde_json::Value,String>) {
    editor.busy=false;
    match result {
        Ok(value)=>{
            if let Some(document)=value.get("document") {editor.program_warning=None;editor.alternate_parts=Project::default();match serde_json::from_value::<Document>(document.clone()) {
                Ok(document)=>{if let Err(error)=document.validate(){editor.error=Some(error);return;}editor.authority=value["owner_id"].as_str().and_then(|owner|uuid::Uuid::parse_str(owner).ok());editor.confirm_outputs=false;editor.selected.clear();editor.baseline=document.clone();editor.document=document;editor.include=Components::default();},Err(e)=>{editor.error=Some(e.to_string());return;}
            }}
            if let Some(report)=value.get("scene_operation") {editor.scene_report=(!report.is_null()).then(||report.clone());editor.next_scene_poll=std::time::Instant::now()+std::time::Duration::from_millis(500);editor.confirm_outputs=false;}
            if let Some(path)=value.get("path").and_then(|p|p.as_str()) {editor.path=path.into();}
            if let Some(warning) = value.get("program_warning") {
                editor.program_warning = warning.as_str().map(str::to_owned);
            }
            editor.notice=Some(if value.get("exported")==Some(&true.into()) {"Component exported locally; project edits remain unsaved."} else if value.get("saved")==Some(&true.into()) {editor.baseline=included(&editor.document,editor.include);"Saved"} else if value.get("media_pending")==Some(&true.into()) {"Loading media; timeline and workspace commit after decoder confirmation."} else if value.get("loaded")==Some(&true.into()) {"Loaded paused. Scene outputs have not been applied."} else if value.get("queued")==Some(&true.into()) {"Hardware commands queued; monitor controller acknowledgements."} else {"Ready"}.into());editor.error=None;
        },Err(error)=>editor.error=Some(error),
    }
}
fn path_field(ui:&mut egui::Ui,label:&str,path:&mut String,key:&str,save:bool) {
    if let Some(value)=crate::ui::peer_browser::take_preference_file(ui.ctx(),key) {*path=value;}
    ui.label(label);
    ui.horizontal(|ui|{
        ui.add(crate::ui::dialog::singleline_text_edit(ui,path).desired_width((ui.available_width()-84.0).max(80.0)));
        if crate::ui::dialog::action_button(ui,crate::ui::icons::FOLDER_OPEN,"Browse").clicked() {
            if crate::peer::active() {crate::ui::peer_browser::open(ui.ctx(),crate::ui::peer_browser::Purpose::PreferenceFile {key:key.into(),extensions:vec!["json".into()]},None);}
            else {
                let dialog=rfd::FileDialog::new().add_filter("Pealayer documents",&["json"]);
                let picked=if save {dialog.set_file_name(if path.is_empty(){"Project.pealayer.json"}else{path.as_str()}).save_file()} else {dialog.pick_file()};
                if let Some(picked)=picked {*path=picked.to_string_lossy().into_owned();}
            }
        }
    });
}
fn switch_component_source<T>(part: &mut Option<Part<T>>, alternate: &mut Option<Part<T>>, linked: bool) {
    let previous = std::mem::replace(part, alternate.take());
    *alternate = previous;
    if linked && part.is_none() {
        *part = Some(Part::File { path: String::new() });
    }
}

fn component<T>(
    ui: &mut egui::Ui, label: &str, key: &str,
    part: &mut Option<Part<T>>, alternate: &mut Option<Part<T>>,
    include: &mut bool, load: &mut bool,
) {
    ui.horizontal(|ui|{
        ui.checkbox(include,label); ui.add_enabled_ui(*include,|ui|{ui.checkbox(load,"Load");});
        if part.is_none() {ui.weak("Not captured");}
        let mut linked=matches!(part,Some(Part::File {..}));
        if ui.checkbox(&mut linked, "Separate file").changed() {
            switch_component_source(part, alternate, linked);
        }
    });
    if let Some(Part::File {path})=part {path_field(ui,"Component file",path,key,false);}
}
fn included(document:&Document,include:Components)->Document {
    let mut value=document.clone();
    if let Document::Project(p)=&mut value {
        if !include.scene {p.scene=None;} if !include.timeline {p.timeline=None;} if !include.media {p.media=None;} if !include.workspace {p.workspace=None;}
    } value
}
pub(crate) fn draw(app:&mut PealayerApp,root:&mut egui::Ui) {
    let ctx=root.ctx().clone();
    let Some(mut editor)=ctx.data_mut(|d|d.get_temp::<Editor>(id())) else {return};
    let reply=editor.reply.lock().ok().and_then(|mut r|r.take());if let Some(reply)=reply {accept(&mut editor,reply);}
    let before=editor.document.clone();
    let selected_before=editor.selected.clone();
    let applying=editor.scene_report.as_ref().is_some_and(|r|r["status"]=="running");
    let mut close=false;let mut discard_close=false;let mut request=None;
    if applying {ctx.request_repaint_after(std::time::Duration::from_millis(250));if !editor.busy && std::time::Instant::now()>=editor.next_scene_poll {request=Some(Request::SceneStatus);}}
    egui::Modal::new(id()).show(&ctx,|ui|{
        ui.set_width((ctx.content_rect().width()-64.0).clamp(300.0,740.0));
        ui.horizontal(|ui|{
            ui.heading(format!("{} Project metadata",crate::ui::icons::FOLDER_OPEN));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{if crate::ui::dialog::close_button(ui,"Close project editor").clicked(){close=true;}});
        });
        ui.separator();
        crate::ui::dialog::scroll_column(ui,"project-editor-content",Some((ctx.content_rect().height()-210.0).max(180.0)),|ui|{
            ui.add_enabled_ui(!editor.busy && !applying,|ui|{
                path_field(ui,"Document file",&mut editor.path,"project-document",true);
                if !editor.candidates.is_empty() {ui.label("Associated projects");for candidate in editor.candidates.clone() {if ui.selectable_label(editor.path==candidate.to_string_lossy(),candidate.file_name().unwrap_or_default().to_string_lossy()).clicked(){editor.path=candidate.to_string_lossy().into_owned();}}}
                ui.horizontal_wrapped(|ui|{
                    if crate::ui::dialog::action_button(ui,crate::ui::icons::FOLDER_OPEN,"Read file").clicked(){request=Some(Request::Read {path:editor.path.clone()});}
                    if ui.button("Capture current session").clicked(){request=Some(Request::Snapshot);}
                    if ui.button("Capture hardware scene").clicked(){request=Some(Request::CaptureScene);}
                });
                ui.separator();
                match &mut editor.document {
                    Document::Project(project)=>{
                        ui.label("Title");ui.add(crate::ui::dialog::singleline_text_edit(ui,&mut project.title).desired_width(f32::INFINITY));
                        ui.label("Notes");ui.add(egui::TextEdit::multiline(&mut project.notes).desired_rows(2).desired_width(f32::INFINITY));
                        ui.label("Save / load components");
                        component(ui,"Scene","project-scene",&mut project.scene,&mut editor.alternate_parts.scene,&mut editor.include.scene,&mut project.load.scene);
                        component(ui,"Timeline","project-timeline",&mut project.timeline,&mut editor.alternate_parts.timeline,&mut editor.include.timeline,&mut project.load.timeline);
                        component(ui,"Media","project-media-info",&mut project.media,&mut editor.alternate_parts.media,&mut editor.include.media,&mut project.load.media);
                        component(ui,"Workspace","project-workspace",&mut project.workspace,&mut editor.alternate_parts.workspace,&mut editor.include.workspace,&mut project.load.workspace);
                        if let Some(Part::Embedded {value:media})=&mut project.media {media_editor(ui,media);}
                        if let Some(Part::Embedded {value:scene})=&mut project.scene {scene_editor(ui,scene,&mut editor.selected);}
                        ui.horizontal_wrapped(|ui|{
                            if ui.button("Export timeline locally").clicked() {if let Some(Part::Embedded {value})=&project.timeline {editor.busy=export(ui,Document::Timeline(value.clone()),editor.reply.clone());}}
                            if ui.button("Export scene locally").clicked() {if let Some(Part::Embedded {value})=&project.scene {editor.busy=export(ui,Document::Scene(value.clone()),editor.reply.clone());}}
                        });
                    },
                    Document::Scene(scene)=>scene_editor(ui,scene,&mut editor.selected),
                    Document::Media(media)=>media_editor(ui,media),
                    Document::Timeline(t)=>{ui.label(format!("{} cues · {} curves · {} timing guides",t.timeline.instances.len(),t.timeline.analog_tracks.len(),t.timeline.keyframes.len()));ui.label("Load this timeline to edit it in the timeline workspace.");},
                    Document::Workspace(_)=>{ui.label("Workspace layout, panels and saved window state");},
                }
                ui.separator();
                ui.checkbox(&mut editor.discard,"Allow replacing unsaved document / session edits");
                ui.weak("Loading restores data and media paused. It never applies scene outputs.");
            });
            if let Some(error)=&editor.error {ui.colored_label(ui.visuals().error_fg_color,error);}
            if let Some(notice)=&editor.notice {ui.label(notice);}
            if let Some(warning) = &editor.program_warning { ui.colored_label(ui.visuals().warn_fg_color, warning); }
            if let Some(report)=&editor.scene_report {
                ui.label(format!("Scene: {}",report["status"].as_str().unwrap_or("unknown")));
                if let Some(outputs)=report["outputs"].as_array() {for output in outputs {ui.label(format!("{} · {}",output["output"]["target"].as_str().unwrap_or("?"),output["status"].as_str().unwrap_or("unknown")));}}
                if applying && ui.add_enabled(!editor.busy,egui::Button::new("Cancel after current output")).clicked() {
                    if let Some(operation_id)=report["id"].as_str().and_then(|id|uuid::Uuid::parse_str(id).ok()) {request=Some(Request::CancelScene{operation_id});}
                }
            }
        });
        ui.separator();
        ui.add_enabled_ui(!editor.busy && !applying,|ui|ui.horizontal_wrapped(|ui|{
            if crate::ui::dialog::primary_action_button(ui,crate::ui::icons::FLOPPY_DISK,"Save").clicked(){request=Some(Request::Write {path:editor.path.clone(),document:included(&editor.document,editor.include)});}
            if ui.button("Load selected components").clicked(){request=Some(Request::Stage {document:included(&editor.document,editor.include),base_path:editor.path.clone(),components:None,discard_unsaved:editor.discard});}
            let scene=match &editor.document {Document::Scene(scene)=>Some(scene),Document::Project(project)=>match &project.scene {Some(Part::Embedded{value})=>Some(value),_=>None},_=>None};
            if let Some(scene)=scene {
                ui.checkbox(&mut editor.confirm_outputs,"Confirm hardware outputs");
                if ui.add_enabled(editor.confirm_outputs && !editor.selected.is_empty(),egui::Button::new("Apply selected outputs")).clicked(){request=Some(Request::ApplyScene {scene:scene.clone(),selected:editor.selected.iter().cloned().collect(),confirm:true});}
            }
            if editor.busy {ui.spinner();}
            if included(&editor.document,editor.include)!=editor.baseline && ui.button("Discard changes and close").clicked(){discard_close=true;}
        }));
    });
    if editor.document!=before || editor.selected!=selected_before {editor.confirm_outputs=false;if matches!(request,Some(Request::ApplyScene{..})){request=None;}}
    if discard_close {ctx.data_mut(|d|d.remove::<Editor>(id()));return;}
    if close {
        if editor.busy || applying {editor.error=Some("Wait for the current document operation to finish, or cancel after the current output.".into());}
        else if included(&editor.document,editor.include)!=editor.baseline {ctx.data_mut(|data|data.insert_temp(id().with("confirm-close"),true));}
        else {ctx.data_mut(|d|d.remove::<Editor>(id()));return;}
    }
    if ctx.data(|data|data.get_temp::<bool>(id().with("confirm-close"))).unwrap_or(false) {
        let response=crate::ui::dialog::confirmation_modal(&ctx,id().with("close-dialog"),"Unsaved document",crate::ui::icons::FLOPPY_DISK,"Save your document before closing, or explicitly discard the changes.","Keep editing",|ui|{
            if ui.button("Keep editing").clicked() {ctx.data_mut(|data|data.remove::<bool>(id().with("confirm-close")));return true;}
            if ui.button("Discard and close").clicked() {discard_close=true;return true;}false
        });
        if response.dismissed || discard_close {ctx.data_mut(|data|data.remove::<bool>(id().with("confirm-close")));}
        if discard_close {ctx.data_mut(|data|data.remove::<Editor>(id()));return;}
    }
    if matches!(request,Some(Request::Read{..}|Request::Snapshot|Request::CaptureScene))
        && included(&editor.document,editor.include)!=editor.baseline && !editor.discard {
        request=None;editor.error=Some("Save the current document or explicitly enable replacement before reading/capturing another.".into());
    }
    ctx.data_mut(|d|d.insert_temp(id(),editor.clone()));
    if let Some(request)=request {
        if crate::peer::active(){queue_remote(&ctx,request);}
        else {
            if request.mutates_authority() && editor.authority != local_authority() {
                reject_changed_authority(&mut editor);
            } else {
                let reply = editor.reply.clone();
                editor.busy = true;
                app.dispatch_document_request(&ctx, request, Some(Box::new(move |result| {
                    if let Ok(mut slot) = reply.lock() {
                        *slot = Some(result);
                    }
                })), None);
                ctx.request_repaint();
            }
            ctx.data_mut(|d|d.insert_temp(id(),editor));
        }
    }
}
fn export(ui:&egui::Ui,document:Document,reply:Arc<Mutex<Option<Result<serde_json::Value,String>>>>)->bool {
    let Some(path)=rfd::FileDialog::new().add_filter("Pealayer documents",&["json"]).save_file() else {return false;};
    // A consumer exports its already captured component to its own filesystem;
    // never send a client-local path to the remote authority or mark the parent
    // project saved merely because one component was exported.
    let ctx=ui.ctx().clone();
    std::thread::spawn(move||{
        let result=document.write(&path).map(|()|serde_json::json!({"exported":true}));
        if let Ok(mut slot)=reply.lock(){*slot=Some(result);}ctx.request_repaint();
    });
    true
}
fn media_editor(ui:&mut egui::Ui,media:&mut Media) {
    egui::CollapsingHeader::new("Media metadata").default_open(true).show(ui,|ui|{
        ui.label("Media path or URL");ui.horizontal(|ui|{
            ui.add(crate::ui::dialog::singleline_text_edit(ui,&mut media.target).desired_width((ui.available_width()-84.0).max(80.0)));
            if ui.button("Browse").clicked(){if crate::peer::active(){crate::ui::peer_browser::open(ui.ctx(),crate::ui::peer_browser::Purpose::PreferenceFile {key:"project-media-target".into(),extensions:vec!["mkv".into(),"mp4".into(),"webm".into(),"mp3".into(),"wav".into()]},None);}else if let Some(path)=rfd::FileDialog::new().pick_file(){media.target=path.to_string_lossy().into_owned();}}
        });
        if let Some(value)=crate::ui::peer_browser::take_preference_file(ui.ctx(),"project-media-target"){media.target=value;}
        ui.horizontal_wrapped(|ui|{ui.label("Position");ui.add(egui::DragValue::new(&mut media.position_ms).range(0..=MAX_TIME_MS).suffix(" ms"));ui.label("Duration");ui.add(egui::DragValue::new(&mut media.duration_ms).range(0..=MAX_TIME_MS).suffix(" ms"));ui.label("Speed");ui.add(egui::DragValue::new(&mut media.speed).range(0.01..=100.0).suffix("×"));});
        ui.weak(format!("{} metadata tags · {} chapters",media.information.metadata.len(),media.information.chapters.len()));
    });
}
fn scene_editor(ui:&mut egui::Ui,scene:&mut Scene,selected:&mut std::collections::BTreeSet<String>) {
    egui::CollapsingHeader::new(format!("Scene · {} channels",scene.channels.len())).default_open(true).show(ui,|ui|{
        ui.label("Name");ui.add(crate::ui::dialog::singleline_text_edit(ui,&mut scene.name).desired_width(f32::INFINITY));
        for (key,value) in &mut scene.channels {ui.push_id(key,|ui|ui.horizontal(|ui|{
            let eligible=scene_output_selectable(key,value);
            let mut checked=selected.contains(key);
            if ui.add_enabled(eligible,egui::Checkbox::new(&mut checked,"Apply")).changed() {if checked {selected.insert(key.clone());}else{selected.remove(key);}}
            ui.label(key);match value {
                ChannelValue::Boolean {on}=>{ui.checkbox(on,"On");},
                ChannelValue::Pwm {percent_milli}=>{let mut percent=f64::from(*percent_milli)/1000.0;if ui.add(egui::DragValue::new(&mut percent).range(0.0..=100.0).suffix(" %")).changed(){*percent_milli=(percent*1000.0).round() as u32;}},
                ChannelValue::Motion {direction}=>{for (v,label) in [(MotionDirection::Stop,"Stop"),(MotionDirection::Up,"Up"),(MotionDirection::Down,"Down")] {ui.selectable_value(direction,v,label);}},
            }
        }));}
        if !scene.unsampled.is_empty(){ui.weak(format!("{} channels had no supported sampled value; they are not changed by this scene.",scene.unsampled.len()));}
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn switching_component_sources_preserves_both_drafts() {
        let original = Media { target: "movie.mkv".into(), ..Media::default() };
        let mut part = Some(Part::embedded(original.clone()));
        let mut alternate = None;
        switch_component_source(&mut part, &mut alternate, true);
        assert_eq!(part, Some(Part::File { path: String::new() }));
        part = Some(Part::File { path: "movie.pealayer-media.json".into() });
        switch_component_source(&mut part, &mut alternate, false);
        assert_eq!(part, Some(Part::embedded(original)));
        switch_component_source(&mut part, &mut alternate, true);
        assert_eq!(part, Some(Part::File { path: "movie.pealayer-media.json".into() }));
    }

    #[test]
    fn discovering_candidates_preserves_the_open_draft_and_pending_reply() {
        let ctx=egui::Context::default();
        open_candidates(&ctx,vec![PathBuf::from("first.json")],None);
        let mut editor=ctx.data_mut(|data|data.get_temp::<Editor>(id())).unwrap();
        if let Document::Project(project)=&mut editor.document {project.notes="unsaved fixture".into();}
        editor.busy=true;let reply=editor.reply.clone();
        ctx.data_mut(|data|data.insert_temp(id(),editor));
        open_candidates(&ctx,vec![PathBuf::from("second.json")],Some("Choose explicitly".into()));
        let retained=ctx.data_mut(|data|data.get_temp::<Editor>(id())).unwrap();
        assert_eq!(retained.path,"first.json");assert!(retained.busy);assert!(Arc::ptr_eq(&reply,&retained.reply));
        assert_eq!(retained.candidates,vec![PathBuf::from("second.json")]);assert!(dirty(&ctx));
        let Document::Project(project)=retained.document else {panic!("draft kind replaced");};
        assert_eq!(project.notes,"unsaved fixture");
    }
}
