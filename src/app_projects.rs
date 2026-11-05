//! Session-authority document adapter shared by native UI, Web, IPC and RPC.
use crate::{app::PealayerApp, projects::*};
use eframe::egui;
use serde_json::{json,Value};
use std::path::{Path,PathBuf};

#[derive(Default)]
pub(crate) struct ProjectSession {
    pub path:Option<PathBuf>,
    pub title:String,
    pub notes:String,
    pub scene:Option<Scene>,
    pub scroll:[f32;2],
    pub track_filter:String,
    pub restore_scroll:Option<[f32;2]>,
    pub pending_media:Option<Media>,
    pending_commit:Option<(Project,PathBuf,bool,egui::Context)>,
    rollback_media:Option<Media>,
    pending_started:Option<std::time::Instant>,
    pending_external_revision:Option<u64>,
    pub pending_discovery:Option<(String,Vec<PathBuf>)>,
    pub bypass_discovery:bool,
    pub discovery_error:Option<String>,
    pub scene_operation:Option<crate::scene_apply::Operation>,
    scene_ticket:Option<crate::scene_apply::Ticket>,
    scene_deadline:Option<std::time::Instant>,
}

impl PealayerApp {
    pub(crate) fn capture_timeline_document(&self)->TimelineDocument {
        let mut timeline=self.timeline.clone();
        timeline.track_muted=self.track_muted.clone(); timeline.track_soloed=self.track_soloed.clone(); timeline.track_locked=self.track_locked.clone();
        timeline.media_target=self.current_video_path.as_ref().map(|p|p.to_string_lossy().into_owned());
        TimelineDocument {timeline,view:TimelineView {position_ms:milliseconds(self.playback_time),zoom:self.timeline_zoom,scroll:self.project.scroll,
            selected_cues:self.selected_instance_ids.iter().copied().collect(),selected_curve_points:self.selected_keyframes.iter().copied().collect(),selected_keyframe:self.selected_timeline_keyframe,
            selected_track:self.selected_timeline_track.clone(),track_filter:self.project.track_filter.clone()}}
    }
    pub(crate) fn capture_project(&self,ctx:&egui::Context)->Project {
        Project {title:self.project.title.clone(),notes:self.project.notes.clone(),load:Components::default(),
            scene:self.project.scene.clone().or_else(||self.advertised_hardware().filter(|c|c.board_connected).map(|c|Scene::capture(&c))).map(Part::embedded),
            timeline:Some(Part::embedded(self.capture_timeline_document())),
            media:self.capture_project_media().map(Part::embedded),
            workspace:Some(Part::embedded(self.capture_workspace_profile(ctx)))}
    }
    fn capture_project_media(&self)->Option<Media> {
        self.current_video_path.as_ref().map(|path|Media {target:path.to_string_lossy().into_owned(),position_ms:milliseconds(self.playback_time),duration_ms:milliseconds(self.duration),paused:self.is_paused,
            speed:self.mpv.get_property::<f64>("speed").unwrap_or(1.0),video_track:track(&self.mpv,"vid"),audio_track:track(&self.mpv,"aid"),subtitle_track:track(&self.mpv,"sid"),information:self.media_file_info.clone()})
    }
    pub(crate) fn document_request(&mut self,ctx:&egui::Context,request:Request)->Result<Value,String> {
        if crate::peer::active() { return Err("Document operations belong to the connected Pealayer authority".into()); }
        match request {
            Request::Validate {document}=>{document.validate()?;Ok(json!({"document":document}))},
            Request::Snapshot=>Ok(json!({"document":Document::Project(self.capture_project(ctx)),"path":self.project.path,"staged_scene":self.project.scene,"scene_operation":self.scene_report()})),
            Request::CaptureScene=>{
                let cap=self.advertised_hardware().filter(|c|c.board_connected).ok_or("No connected board has sampled channel values")?;
                let scene=Scene::capture(&cap);scene.validate()?;
                Ok(json!({"document":Document::Scene(scene)}))
            },
            Request::Read {path}=>Ok(json!({"document":Document::read(Path::new(&path))?,"path":path})),
            Request::Write {path,document}=>{
                document.write(Path::new(&path))?;
                if let Document::Project(project)=&document {
                    self.project.path=Some(PathBuf::from(&path));self.project.title=project.title.clone();self.project.notes=project.notes.clone();
                    // Only mark the timeline saved if the written embedded data is identical.
                    if let Some(Part::Embedded {value})=&project.timeline {
                        if value.timeline==self.capture_timeline_document().timeline {self.timeline_saved_baseline=Some(self.snapshot_timeline());}
                    }
                    self.remember_project(Path::new(&path),project);
                } else if let Document::Timeline(t)=&document {
                    if t.timeline==self.capture_timeline_document().timeline {self.timeline_saved_baseline=Some(self.snapshot_timeline());}
                }
                Ok(json!({"saved":true,"path":path}))
            },
            Request::Load {path,components,discard_unsaved}=>{
                let document=Document::read(Path::new(&path))?;
                self.stage_document(ctx,document,Path::new(&path),components,discard_unsaved)
            },
            Request::Stage {document,base_path,components,discard_unsaved}=>self.stage_document(ctx,document,Path::new(&base_path),components,discard_unsaved),
            Request::ApplyScene {scene,selected,confirm}=>{
                if !confirm {return Err("Explicit confirmation is required to apply scene outputs".into());}
                self.apply_scene(&scene,&selected)?;
                self.poll_scene_operation();
                Ok(json!({"scene_operation":self.scene_report()}))
            },
            Request::SceneStatus=>Ok(json!({"scene_operation":self.scene_report()})),
            Request::CancelScene {operation_id}=>{
                let operation=self.project.scene_operation.as_mut().filter(|op|op.id()==operation_id).ok_or("Scene operation no longer exists")?;
                operation.cancel(std::time::Instant::now());
                Ok(json!({"scene_operation":self.scene_report()}))
            },
        }
    }
    fn stage_document(&mut self,ctx:&egui::Context,document:Document,path:&Path,components:Option<Components>,discard:bool)->Result<Value,String> {
        if self.project.pending_media.is_some() {return Err("Wait for the pending project media load or cancel it by opening another file".into());}
        document.validate()?;
        let is_project=matches!(&document,Document::Project(_));
        let project=match document {
            Document::Project(p)=>p,
            Document::Scene(value)=>Project {scene:Some(Part::embedded(value)),..Project::default()},
            Document::Timeline(value)=>Project {timeline:Some(Part::embedded(value)),..Project::default()},
            Document::Media(value)=>Project {media:Some(Part::embedded(value)),..Project::default()},
            Document::Workspace(value)=>Project {workspace:Some(Part::embedded(value)),..Project::default()},
        };
        let loaded=resolve(&project,path,components.unwrap_or(project.load))?;
        if loaded.timeline.is_some() || loaded.workspace.is_some() || loaded.media.is_some() {
            let inspection=self.inspect_authoring(ctx);
            if inspection.items.iter().any(|i|i.state==crate::authoring::State::Busy) {return Err("Finish pending recording or hardware operations before loading a document".into());}
            if !discard && inspection.items.iter().any(|i|matches!(i.state,crate::authoring::State::Dirty|crate::authoring::State::Unknown)) {
                return Err("Unsaved or unconfirmed editors exist. Save them, or explicitly confirm replacing the session".into());
            }
            // Restoring data never starts a prepared hardware plan or previews a scrub.
            self.mpv.set_property("pause",true).map_err(|e|format!("Pause before loading project: {e}"))?;
            self.is_paused=true;
            self.was_playing_before_scrub=false;
            if let Ok(mut plan)=self.engine_handle.prepared_timeline.lock() {plan.cancel_play();}
            self.engine_handle.is_playing.store(false,std::sync::atomic::Ordering::Release);
        }
        if let Some(Part::Embedded {value:media})=&loaded.media {
            let previous_media=self.capture_project_media();
            let external_revision=crate::mpv::external::active().then(||crate::mpv::external::status().loaded_revision);
            // Supply start with loadfile, before decode. FileLoaded reasserts the
            // exact target (including positions below one second) and track selection.
            let options=format!("pause=yes,start={},speed={}",media.position_ms as f64/1000.0,media.speed);
            self.mpv.command("loadfile",&[&media.target,"replace","-1",&options]).map_err(|e|format!("Load project media: {e}"))?;
            self.show_error=None;
            self.reset_project_media_state();
            self.current_video_path=Some(PathBuf::from(&media.target));self.last_media_target=self.current_video_path.clone();
            self.pending_resume_position=None; self.project.pending_media=Some(media.clone());
            self.project.rollback_media=previous_media;
            self.project.pending_started=Some(std::time::Instant::now());
            self.project.pending_external_revision=external_revision;
            self.playback_time=media.position_ms as f64/1000.0;self.duration=0.0;self.is_seekable=false;self.is_eof=false;
            self.media_file_info=crate::media_info::MediaFileInfo::default();
            self.project.pending_commit=Some((loaded,path.to_path_buf(),is_project,ctx.clone()));
            return Ok(json!({"loaded":false,"media_pending":true,"scene_applied":false,"playback":"paused"}));
        }
        // Seek can fail; do it before committing any authoring/workspace state.
        if let Some(Part::Embedded {value:timeline})=&loaded.timeline {
            let seconds=timeline.view.position_ms as f64/1000.0;
            if self.is_seekable {self.mpv.command("seek",&[&seconds.to_string(),"absolute+exact"]).map_err(|e|format!("Restore timeline playhead: {e}"))?;}
        }
        self.commit_project(ctx,loaded,path,is_project)?;
        Ok(json!({"loaded":true,"scene_applied":false,"playback":"paused","media_pending":false}))
    }
    fn commit_project(&mut self,ctx:&egui::Context,loaded:Project,path:&Path,is_project:bool)->Result<(),String> {
        let metadata=loaded.clone();
        if let Some(Part::Embedded {value:workspace})=&loaded.workspace {self.apply_workspace_profile(ctx,workspace)?;}
        if let Some(Part::Embedded {value:timeline})=loaded.timeline {
            self.undo_stack.push(self.snapshot_timeline());
            self.timeline=timeline.timeline;
            self.track_muted=self.timeline.track_muted.clone();self.track_soloed=self.timeline.track_soloed.clone();self.track_locked=self.timeline.track_locked.clone();
            self.timeline_zoom=timeline.view.zoom;self.project.restore_scroll=Some(timeline.view.scroll);self.project.scroll=timeline.view.scroll;
            self.selected_instance_ids=timeline.view.selected_cues.into_iter().collect();self.selected_keyframes=timeline.view.selected_curve_points.into_iter().collect();self.selected_timeline_keyframe=timeline.view.selected_keyframe;
            self.selected_timeline_track=timeline.view.selected_track;
            self.project.track_filter=timeline.view.track_filter.clone();
            ctx.data_mut(|data|data.insert_temp(egui::Id::new("timeline_track_filter_text"),timeline.view.track_filter));
            if loaded.media.is_none() {
                let seconds=timeline.view.position_ms as f64/1000.0;
                self.playback_time=seconds;
            }
            self.sync_timeline_engine();self.timeline_saved_baseline=Some(self.snapshot_timeline());
        }
        if let Some(Part::Embedded {value:scene})=loaded.scene {self.project.scene=Some(scene);}
        if is_project {
            self.project.path=(!path.as_os_str().is_empty()).then(||path.to_path_buf());self.project.title=metadata.title.clone();self.project.notes=metadata.notes.clone();
            if !path.as_os_str().is_empty() {self.remember_project(path,&metadata);}
        }
        ctx.request_repaint();
        Ok(())
    }
    fn remember_project(&mut self,path:&Path,project:&Project) {
        let Some(Part::Embedded {value:media})=&project.media else {return};
        let path=std::fs::canonicalize(path).unwrap_or_else(|_|path.to_path_buf());
        let mut config=self.runtime_config_snapshot();
        if config.project_discovery.associations.len()>=1000 && !config.project_discovery.associations.contains_key(&media.target) {return;}
        config.project_discovery.associations.insert(media.target.clone(),path.to_string_lossy().into_owned());
        match config.save() {
            Ok(())=>crate::platform::interop::set_live_config(config),
            Err(error)=>self.show_error=Some(format!("Document is saved/loaded, but its media association could not be saved: {error}")),
        }
    }
    pub(crate) fn finish_project_media_load(&mut self) {
        let Some(expected)=self.project.pending_media.as_ref() else {return;};
        if let Some(before)=self.project.pending_external_revision {
            let status=crate::mpv::external::status();
            if !status.connected || status.loaded_revision<=before {return;}
            self.is_seekable=status.properties.get("seekable").and_then(Value::as_bool).unwrap_or(false);
        }
        // Queued FileLoaded from the preceding decoder must not commit this
        // project's authoring state. A different explicit Open cancels it too.
        let current=self.mpv.get_property::<String>("path").unwrap_or_default();
        if !same_media_target(&current,&expected.target) {return;}
        let Some(media)=self.project.pending_media.take() else {return};
        self.pending_resume_position=None;
        let result=(||{
            self.mpv.set_property("pause",true).map_err(|e|e.to_string())?;
            for (property,value) in [("vid",&media.video_track),("aid",&media.audio_track),("sid",&media.subtitle_track)] {
                self.mpv.set_property(property,value.as_str()).map_err(|e|e.to_string())?;
            }
            if self.is_seekable {
                let target=media.position_ms as f64/1000.0;
                self.mpv.command("seek",&[&target.to_string(),"absolute+exact"]).map_err(|e|e.to_string())?;
                self.playback_time=target;
            } else if media.position_ms>0 {return Err("This media cannot seek to the project position".into());}
            Ok::<(),String>(())
        })();
        self.is_paused=true;
        if let Err(e)=result {self.fail_project_media_load(&e);}
        else {
            self.project.pending_started=None;self.project.rollback_media=None;self.project.pending_external_revision=None;
            if let Some((loaded,path,is_project,ctx))=self.project.pending_commit.take() {
                self.add_recent_media(PathBuf::from(&media.target));
                if let Err(error)=self.commit_project(&ctx,loaded,&path,is_project) {self.show_error=Some(error);}
            }
        }
    }
    pub(crate) fn cancel_project_media_load(&mut self) {self.project.pending_media=None;self.project.pending_commit=None;self.project.rollback_media=None;self.project.pending_started=None;self.project.pending_external_revision=None;}
    pub(crate) fn fail_project_media_load(&mut self,error:&str) {
        let previous=self.project.rollback_media.take();
        self.cancel_project_media_load();
        self.is_paused=true;
        self.show_error=Some(format!("Project media restoration: {error}. Existing timeline/workspace retained; playback remains paused."));
        if let Some(media)=previous {
            let external_revision=crate::mpv::external::active().then(||crate::mpv::external::status().loaded_revision);
            let options=format!("pause=yes,start={},speed={}",media.position_ms as f64/1000.0,media.speed);
            if self.mpv.command("loadfile",&[&media.target,"replace","-1",&options]).is_ok() {
                self.current_video_path=Some(PathBuf::from(&media.target));self.last_media_target=self.current_video_path.clone();
                self.playback_time=media.position_ms as f64/1000.0;
                self.project.pending_media=Some(media);self.project.pending_started=Some(std::time::Instant::now());
                self.project.pending_external_revision=external_revision;
                return;
            }
            self.show_error=Some(format!("Project restoration failed: {error}; reopening the previous media also failed. Timeline/workspace retained."));
        }
        let _=self.mpv.command("stop",&[]);
        self.current_video_path=None;self.playback_time=0.0;self.duration=0.0;
    }
    pub(crate) fn discover_media_project(&mut self,target:&str)->bool {
        if self.project.bypass_discovery || crate::peer::active() {return false;}
        let candidates=crate::platform::interop::get_live_config().project_discovery.candidates(target);
        if candidates.is_empty() {return false;}
        self.project.pending_discovery=Some((target.into(),candidates));true
    }
    pub(crate) fn poll_project_discovery(&mut self,ctx:&egui::Context) {
        self.poll_scene_operation();
        if self.scene_operation_busy() {ctx.request_repaint_after(std::time::Duration::from_millis(100));}
        if self.project.pending_external_revision.is_some() {self.finish_project_media_load();}
        if let Some(started)=self.project.pending_started {
            if started.elapsed()>=std::time::Duration::from_secs(30) {self.fail_project_media_load("decoder confirmation timed out");}
            else {ctx.request_repaint_after(std::time::Duration::from_millis(250));}
        }
        let Some((target,candidates))=self.project.pending_discovery.take() else {return};
        if candidates.len()==1 {
            let result=discovered_project(&candidates[0],&target).and_then(|project|
                self.stage_document(ctx,Document::Project(project),&candidates[0],None,false));
            if result.is_ok() {return;}
            self.project.discovery_error=result.err();
        } else {self.project.discovery_error=Some("Multiple associated projects found. Choose which project to open.".into());}
        crate::ui::projects::open_candidates(ctx,candidates,self.project.discovery_error.clone());
        // Do not silently replace the requested media with another project's
        // media, discard edits, or recurse into association discovery on failure.
        self.project.bypass_discovery=true;self.load_media_target(&target);self.project.bypass_discovery=false;
    }
    pub(crate) fn scene_operation_busy(&self)->bool {
        self.project.scene_ticket.is_some() || self.project.scene_operation.as_ref().is_some_and(|operation|!operation.is_finished())
    }
    fn scene_report(&self)->Option<crate::scene_apply::Report> {self.project.scene_operation.as_ref().map(|op|op.report())}
    fn scene_interlocks_clear(&self)->bool {
        self.is_paused && !self.estop_active && self.board_operation.is_none() && !self.hardware_effect_authoring.active
            && self.hardware_effect_authoring.pending_operation.is_none() && !self.is_recording
            && self.project.pending_media.is_none() && !crate::peer::active()
    }
    fn apply_scene(&mut self,scene:&Scene,selected:&[String])->Result<(),String> {
        if self.scene_operation_busy() {return Err("A scene operation is already running".into());}
        if !self.scene_interlocks_clear() {return Err("Pause playback and finish pending media/board operations; E-STOP must be clear".into());}
        let cap=self.advertised_hardware().filter(|c|c.board_connected).ok_or("No connected board")?;
        let plan=crate::scene_apply::preflight(scene,selected,&cap,Some(&cap.pwm_configuration_revision)).map_err(|e|e.to_string())?;
        let now=std::time::Instant::now();
        self.project.scene_operation=Some(crate::scene_apply::Operation::new(plan,now));
        self.project.scene_ticket=None;self.project.scene_deadline=Some(now+crate::scene_apply::DEADLINE);
        Ok(())
    }
    fn poll_scene_operation(&mut self) {
        if !self.scene_operation_busy() {return;}
        let now=std::time::Instant::now();
        let cap=self.advertised_hardware().unwrap_or_default();
        let safe=self.scene_interlocks_clear();
        let Some(operation)=self.project.scene_operation.as_mut() else {return;};
        if !safe {operation.cancel(now);}
        let request=operation.next_request(now,&cap,Some(&cap.pwm_configuration_revision));
        if let Some(request)=request {
            let key=format!("project-scene:{}:{}",request.ticket.operation_id,request.ticket.index);
            self.project.scene_ticket=Some(request.ticket);
            if self.engine_handle.request_controller_call_before(key,request.method,request.params,self.project.scene_deadline).is_err() {
                operation.complete(request.ticket,crate::scene_apply::Completion::Rejected,now);
                self.project.scene_ticket=None;
            }
        }
    }
    pub(crate) fn complete_scene_call(&mut self,result:crate::four_d::engine::ControllerCallResult) {
        let Some(ticket)=self.project.scene_ticket else {return;};
        if result.operation!=format!("project-scene:{}:{}",ticket.operation_id,ticket.index) {return;}
        let completion=match &result.result {
            Ok(value)=>crate::scene_apply::Completion::from_success(value),
            Err(_)=>crate::scene_apply::Completion::from_error_data(result.error_detail.as_ref()),
        };
        if let Some(operation)=self.project.scene_operation.as_mut() {operation.complete(ticket,completion,std::time::Instant::now());}
        self.project.scene_ticket=None;
        self.poll_scene_operation();
    }
}
fn milliseconds(value:f64)->u64 {if value.is_finite() { (value.max(0.0)*1000.0).round().min(MAX_TIME_MS as f64) as u64 } else {0}}
fn track(mpv:&crate::mpv::player::Player,key:&str)->String {mpv.get_property::<String>(key).ok().filter(|s|matches!(s.as_str(),"auto"|"no")||s.parse::<u32>().is_ok()).unwrap_or_else(||"auto".into())}
