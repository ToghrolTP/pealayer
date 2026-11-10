//! Session-authority document adapter shared by native UI, Web, IPC and RPC.
use crate::{app::PealayerApp, projects::*};
use eframe::egui;
use serde_json::{json,Value};
use std::path::{Path,PathBuf};

struct ProjectSeek {
    request_id: u64,
    target: String,
}

impl ProjectSeek {
    fn confirmed(&self, request_id: u64, target: &str, paused: bool) -> bool {
        request_id == self.request_id && paused && same_media_target(&self.target, target)
    }
}

#[derive(Clone, Copy)]
struct ExternalLoadFence {
    loaded: u64,
    failed: u64,
}

impl ExternalLoadFence {
    fn active() -> Option<Self> {
        crate::mpv::external::active().then(|| {
            let status = crate::mpv::external::status();
            Self { loaded: status.loaded_revision, failed: status.load_failure_revision }
        })
    }
}

pub(crate) type DocumentReply = Box<dyn FnOnce(Result<Value, String>) + Send>;

struct DocumentWork {
    prepared: std::sync::mpsc::Receiver<Result<PreparedDocument, String>>,
    reply: Option<DocumentReply>,
    restoring: bool,
    superseded: bool,
    deadline: Option<std::time::Instant>,
}

enum PreparedDocument {
    Read { path: String, document: Document },
    Write { path: PathBuf, document: Document },
    Restore { path: PathBuf, project: Project, is_project: bool, discard: bool },
}

fn prepare_file_request(request: Request) -> Result<PreparedDocument, String> {
    match request {
        Request::Read { path } => Ok(PreparedDocument::Read { document: Document::read(Path::new(&path))?, path }),
        Request::Write { path, document } => {
            let path = PathBuf::from(path);
            document.write(&path)?;
            Ok(PreparedDocument::Write { path, document })
        }
        Request::Load { path, components, discard_unsaved } => {
            let path = PathBuf::from(path);
            prepare_restore(Document::read(&path)?, path, components, discard_unsaved)
        }
        Request::Stage { document, base_path, components, discard_unsaved } =>
            prepare_restore(document, PathBuf::from(base_path), components, discard_unsaved),
        _ => unreachable!("live operations do not use the document file worker"),
    }
}

fn prepare_restore(document: Document, path: PathBuf, components: Option<Components>, discard: bool) -> Result<PreparedDocument, String> {
    let is_project = matches!(&document, Document::Project(_));
    let project = match document {
        Document::Project(value) => value,
        Document::Scene(value) => Project { scene: Some(Part::embedded(value)), ..Project::default() },
        Document::Timeline(value) => Project { timeline: Some(Part::embedded(value)), ..Project::default() },
        Document::Media(value) => Project { media: Some(Part::embedded(value)), ..Project::default() },
        Document::Workspace(value) => Project { workspace: Some(Part::embedded(value)), ..Project::default() },
    };
    let project = resolve(&project, &path, components.unwrap_or(project.load))?;
    Ok(PreparedDocument::Restore { path, project, is_project, discard })
}

#[derive(Default)]
pub(crate) struct ProjectSession {
    document_work: Option<DocumentWork>,
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
    pending_external_revision:Option<ExternalLoadFence>,
    native_start_seen: bool,
    pending_seek: Option<ProjectSeek>,
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
    pub(crate) fn save_timeline_file(&mut self, path: &Path) -> Result<(), String> {
        Document::Timeline(self.capture_timeline_document()).write(path)?;
        self.timeline_saved_baseline = Some(self.snapshot_timeline());
        let name = path.file_name().and_then(|name| name.to_str()).unwrap_or("timeline");
        self.set_osd(format!("{} {name}", self.tr("Timeline saved:")));
        Ok(())
    }

    pub(crate) fn open_timeline_file(&mut self, ctx: &egui::Context, path: &Path) -> Result<Value, String> {
        if self.project_restore_busy() {
            return Err("Wait for the pending project restoration before opening a timeline".into());
        }
        if self.is_timeline_dirty() {
            return Err("Save the current timeline before opening another timeline".into());
        }
        match read_timeline_file(path)? {
            TimelineFile::Document(timeline) => {
                self.stage_document(ctx, Document::Timeline(timeline), path, None, false)
            }
            TimelineFile::Sidecar(timeline) => {
                // The raw sidecar has no saved view. Retain its media-opening
                // behaviour and never interpret its missing position as zero.
                if self.current_video_path.is_none()
                    && let Some(target) = &timeline.media_target
                    && Path::new(target).is_file()
                {
                    let previous = self.project.bypass_discovery;
                    self.project.bypass_discovery = true;
                    self.load_video_file(PathBuf::from(target));
                    self.project.bypass_discovery = previous;
                }
                self.undo_stack.push(self.snapshot_timeline());
                self.track_muted = timeline.track_muted.clone();
                self.track_soloed = timeline.track_soloed.clone();
                self.track_locked = timeline.track_locked.clone();
                self.timeline = timeline;
                self.sync_timeline_engine();
                self.timeline_saved_baseline = Some(self.snapshot_timeline());
                Ok(json!({"loaded": true}))
            }
        }
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
    pub(crate) fn dispatch_document_request(
        &mut self,
        ctx: &egui::Context,
        request: Request,
        reply: Option<DocumentReply>,
        deadline: Option<std::time::Instant>,
    ) {
        if crate::peer::active() {
            self.reply_to_document(reply, Err("Document operations belong to the connected Pealayer authority".into()));
            return;
        }
        if self.project.document_work.is_some() && matches!(&request, Request::ApplyScene { .. }) {
            self.reply_to_document(reply, Err("Wait for the current document file operation to finish".into()));
            return;
        }
        match request {
            request @ (Request::Read { .. } | Request::Write { .. } | Request::Load { .. } | Request::Stage { .. }) => {
                if self.project.document_work.is_some() {
                    self.reply_to_document(reply, Err("Wait for the current document file operation to finish".into()));
                    return;
                }
                let restoring = matches!(&request, Request::Load { .. } | Request::Stage { .. });
                let (sender, prepared) = std::sync::mpsc::channel();
                let context = ctx.clone();
                let work = std::thread::Builder::new().name("project-document".into()).spawn(move || {
                    let result = prepare_file_request(request);
                    let _ = sender.send(result);
                    context.request_repaint();
                });
                match work {
                    Ok(_) => self.project.document_work = Some(DocumentWork { prepared, reply, restoring, superseded: false, deadline }),
                    Err(error) => self.reply_to_document(reply, Err(format!("Could not start document file operation: {error}"))),
                }
            }
            request => {
                let result = self.handle_live_document_request(ctx, request);
                self.reply_to_document(reply, result);
            }
        }
    }

    fn reply_to_document(&mut self, reply: Option<DocumentReply>, result: Result<Value, String>) {
        let result = result.map(|mut value| {
            value["owner_id"] = Value::String(crate::peer::instance_id().into());
            value
        });
        if let Some(reply) = reply {
            reply(result);
        } else if let Err(error) = result {
            self.show_error = Some(error);
        }
    }

    pub(crate) fn document_io_busy(&self) -> bool {
        self.project.document_work.is_some()
    }

    fn poll_document_work(&mut self, ctx: &egui::Context) {
        use std::sync::mpsc::TryRecvError;
        let Some(work) = &self.project.document_work else { return; };
        let prepared = match work.prepared.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err("Document file operation stopped before completion".into()),
        };
        let work = self.project.document_work.take().expect("document worker was present");
        let result = if work.superseded {
            Err("A newer media action replaced the pending document restore".into())
        } else if work.restoring && work.deadline.is_some_and(|deadline| deadline <= std::time::Instant::now()) {
            Err("Request expired before the document could be restored".into())
        } else if crate::peer::active() {
            Err("Document authority changed before the result could be applied".into())
        } else {
            prepared.and_then(|prepared| match prepared {
                PreparedDocument::Read { path, document } => Ok(json!({"document": document, "path": path})),
                PreparedDocument::Write { path, document } => {
                    self.finish_document_write(&path, &document);
                    Ok(json!({"saved": true, "path": path}))
                }
                PreparedDocument::Restore { path, project, is_project, discard } =>
                    self.stage_resolved_project(ctx, project, &path, is_project, discard),
            })
        };
        self.reply_to_document(work.reply, result);
    }

    fn finish_document_write(&mut self, path: &Path, document: &Document) {
        let timeline = match document {
            Document::Project(project) => {
                self.project.path = Some(path.to_owned());
                self.project.title.clone_from(&project.title);
                self.project.notes.clone_from(&project.notes);
                self.remember_project(path, project);
                match &project.timeline {
                    Some(Part::Embedded { value }) => Some(value),
                    _ => None,
                }
            }
            Document::Timeline(timeline) => Some(timeline),
            _ => None,
        };
        if timeline.is_some_and(|saved| saved.timeline == self.capture_timeline_document().timeline) {
            self.timeline_saved_baseline = Some(self.snapshot_timeline());
        }
    }

    fn handle_live_document_request(&mut self, ctx: &egui::Context, request: Request) -> Result<Value, String> {
        match request {
            Request::Validate {document}=>{document.validate()?;Ok(json!({"document":document}))},
            Request::Snapshot=>Ok(json!({"document":Document::Project(self.capture_project(ctx)),"path":self.project.path,"staged_scene":self.project.scene,"scene_operation":self.scene_report()})),
            Request::CaptureScene=>{
                let cap=self.advertised_hardware().filter(|c|c.board_connected).ok_or("No connected board has sampled channel values")?;
                let scene=Scene::capture(&cap);scene.validate()?;
                Ok(json!({"document":Document::Scene(scene)}))
            },
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
            _ => unreachable!("file operations are prepared by the document worker"),
        }
    }

    fn stage_document(&mut self, ctx: &egui::Context, document: Document, path: &Path, components: Option<Components>, discard: bool) -> Result<Value, String> {
        let PreparedDocument::Restore { path, project, is_project, discard } =
            prepare_restore(document, path.to_owned(), components, discard)? else { unreachable!() };
        self.stage_resolved_project(ctx, project, &path, is_project, discard)
    }

    fn stage_resolved_project(&mut self,ctx:&egui::Context,loaded:Project,path:&Path,is_project:bool,discard:bool)->Result<Value,String> {
        if self.project_restore_busy() {return Err("Wait for the pending project media load or cancel it by opening another file".into());}
        if self.scene_operation_busy() {return Err("Finish the pending scene output operation before loading a document".into());}
        if loaded.media.is_some() {
            // Drain older decoder events before pausing or arming the new
            // transaction. An ordinary load can still deliver a same-path event.
            self.process_events();
            if self.current_video_path.is_some() && !self.media_metadata_loaded && !crate::mpv::external::active() {
                return Err("Wait for the current media load before restoring a project".into());
            }
        }
        let program_warning = self.project_program_warning(&loaded);
        if loaded.timeline.is_some() || loaded.workspace.is_some() || loaded.media.is_some() {
            let inspection=self.inspect_authoring(ctx);
            if inspection.items.iter().any(|i|i.domain != "project_operation" && i.state==crate::authoring::State::Busy) {return Err("Finish pending recording or hardware operations before loading a document".into());}
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
            let external_revision=ExternalLoadFence::active();
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
            self.project.native_start_seen = false;
            self.playback_time=media.position_ms as f64/1000.0;self.duration=0.0;self.is_seekable=false;self.is_eof=false;
            self.media_file_info=crate::media_info::MediaFileInfo::default();
            self.project.pending_commit=Some((loaded,path.to_path_buf(),is_project,ctx.clone()));
            return Ok(pending_project_result(program_warning));
        }
        if let Some(Part::Embedded { value: timeline }) = &loaded.timeline
            && self.is_seekable
        {
            self.queue_project_seek(timeline.view.position_ms)?;
            self.project.pending_commit = Some((loaded, path.to_path_buf(), is_project, ctx.clone()));
            self.project.pending_started = Some(std::time::Instant::now());
            return Ok(pending_project_result(program_warning));
        }
        self.commit_project(ctx,loaded,path,is_project)?;
        Ok(json!({"loaded":true,"scene_applied":false,"playback":"paused","media_pending":false,"program_warning":program_warning}))
    }
    fn project_program_warning(&self, project: &Project) -> Option<String> {
        let Some(Part::Embedded { value: timeline }) = &project.timeline else { return None; };
        let capabilities = self.advertised_hardware_shared();
        let catalog = capabilities.as_deref().filter(|cap| self.is_connected && cap.board_connected);
        missing_controller_programs(&timeline.timeline, catalog)
    }

    fn commit_project(&mut self,ctx:&egui::Context,loaded:Project,path:&Path,is_project:bool)->Result<(),String> {
        let program_warning = self.project_program_warning(&loaded);
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
            if loaded.media.is_none() && self.current_video_path.is_none() {
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
        if let Some(warning) = program_warning { self.set_osd(warning); }
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
    pub(crate) fn project_restore_busy(&self) -> bool {
        self.project.pending_media.is_some() || self.project.pending_seek.is_some()
    }

    pub(crate) fn note_project_media_start(&mut self) {
        if self.project.pending_media.is_some() && self.project.pending_external_revision.is_none() {
            self.project.native_start_seen = true;
        }
    }

    pub(crate) fn project_decoder_failed(&self) -> bool {
        self.project.pending_seek.is_some() ||
            (self.project.pending_media.is_some() && self.project.native_start_seen)
    }

    fn decoder_media_target(&self) -> String {
        let path = self.mpv.get_property::<String>("path").unwrap_or_default();
        if crate::mpv::external::active() && !path.is_empty() {
            crate::mpv::external::resolve_path(&crate::mpv::external::status(), &path)
        } else {
            path
        }
    }

    fn queue_project_seek(&mut self, position_ms: u64) -> Result<(), String> {
        let target = self.decoder_media_target();
        if target.is_empty() || !self.mpv.get_property::<bool>("pause").unwrap_or(false) {
            return Err("The active decoder did not confirm paused project media".into());
        }
        let seconds = position_ms as f64 / 1000.0;
        if self.duration > 0.0 && seconds > self.duration {
            return Err("Saved project position is beyond the decoded media duration".into());
        }
        self.finish_scrub(seconds);
        let request_id = self.pending_exact_seek_id().ok_or("Could not queue an exact project seek")?;
        self.project.pending_seek = Some(ProjectSeek { request_id, target });
        Ok(())
    }

    pub(crate) fn finish_project_media_load(&mut self) {
        let Some(expected) = self.project.pending_media.as_ref() else { return; };
        if self.project.pending_seek.is_some() { return; }
        if let Some(before) = self.project.pending_external_revision {
            let status = crate::mpv::external::status();
            if !status.connected { return; }
            if status.load_failure_revision > before.failed {
                self.fail_project_media_load("external decoder rejected the media");
                return;
            }
            if status.loaded_revision <= before.loaded { return; }
            // FileLoaded and property observations travel separately over IPC.
            // Wait for the new file's seekability and clock before committing.
            let Some(seekable) = status.properties.get("seekable").and_then(Value::as_bool) else { return; };
            let duration = status.properties.get("duration").and_then(Value::as_f64);
            if expected.position_ms > 0 && !seekable
                || seekable && expected.duration_ms > 0 && duration.is_none()
                || !seekable && status.properties.get("time-pos").and_then(Value::as_f64).is_none()
            {
                return;
            }
            self.is_seekable = seekable;
            self.duration = duration.unwrap_or(0.0);
        } else if !self.project.native_start_seen {
            return;
        }
        if !same_media_target(&self.decoder_media_target(), &expected.target) { return; }

        let position_ms = expected.position_ms;
        let result = (|| {
            self.mpv.set_property("pause", true).map_err(|e| e.to_string())?;
            for (property, value) in [
                ("vid", &expected.video_track),
                ("aid", &expected.audio_track),
                ("sid", &expected.subtitle_track),
            ] {
                self.mpv.set_property(property, value.as_str()).map_err(|e| e.to_string())?;
            }
            Ok::<_, String>(())
        })();
        self.pending_resume_position = None;
        self.is_paused = true;
        if let Err(error) = result {
            self.fail_project_media_load(&error);
            return;
        }
        if self.is_seekable {
            if let Err(error) = self.queue_project_seek(position_ms) {
                self.fail_project_media_load(&error);
            }
        } else if position_ms == 0
            && self.mpv.get_property::<f64>("time-pos").is_ok_and(|position| (0.0..0.0005).contains(&position))
            && self.mpv.get_property::<bool>("pause").unwrap_or(false)
        {
            self.finish_project_restore();
        } else {
            self.fail_project_media_load("decoder could not confirm the saved position");
        }
    }

    pub(crate) fn project_seek_settled(&mut self, request_id: u64) {
        let Some(seek) = &self.project.pending_seek else { return; };
        if seek.request_id != request_id { return; }
        if !seek.confirmed(request_id, &self.decoder_media_target(), self.mpv.get_property("pause").unwrap_or(false)) {
            self.fail_project_media_load("decoder authority or pause state changed");
            return;
        }
        self.finish_project_restore();
    }

    fn finish_project_restore(&mut self) {
        let media = self.project.pending_media.take();
        let commit = self.project.pending_commit.take();
        self.cancel_project_media_load();
        if let Some((loaded, path, is_project, ctx)) = commit {
            if let Some(media) = media { self.add_recent_media(PathBuf::from(media.target)); }
            if let Err(error) = self.commit_project(&ctx, loaded, &path, is_project) {
                self.show_error = Some(error);
            }
        }
    }

    pub(crate) fn cancel_project_media_load(&mut self) {
        if let Some(work) = self.project.document_work.as_mut().filter(|work| work.restoring) {
            work.superseded = true;
        }
        if let Some(seek) = self.project.pending_seek.take()
            && self.pending_exact_seek_id() == Some(seek.request_id)
        {
            self.reset_scrub_state();
        }
        self.project.pending_media = None;
        self.project.pending_commit = None;
        self.project.rollback_media = None;
        self.project.pending_started = None;
        self.project.pending_external_revision = None;
        self.project.native_start_seen = false;
    }

    pub(crate) fn fail_project_media_load(&mut self,error:&str) {
        let replacing_media = self.project.pending_media.is_some();
        let previous=self.project.rollback_media.take();
        self.cancel_project_media_load();
        self.is_paused=true;
        self.show_error=Some(format!("Project media restoration: {error}. Existing timeline/workspace retained; playback remains paused."));
        if !replacing_media { return; }
        if let Some(media)=previous {
            let external_revision=ExternalLoadFence::active();
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
        self.poll_document_work(ctx);
        self.poll_scene_operation();
        if self.scene_operation_busy() {ctx.request_repaint_after(std::time::Duration::from_millis(100));}
        if self.project.pending_external_revision.is_some() {self.finish_project_media_load();}
        if self.project.pending_seek.as_ref().is_some_and(|seek| self.pending_exact_seek_id() != Some(seek.request_id)) {
            self.fail_project_media_load("the exact project seek was rejected or superseded");
        }
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
            && !self.project_restore_busy() && !crate::peer::active()
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
        if let Some(operation) = self.project.scene_operation.as_mut() {
            operation.complete(ticket, completion, std::time::Instant::now());
            if operation.is_finished() {
                // IPC-only coordinators may not push output telemetry. Refresh
                // once at completion so capture sees current sampled values.
                self.engine_handle.request_catalog_refresh();
            }
        }
        self.project.scene_ticket=None;
        self.poll_scene_operation();
    }
}
fn pending_project_result(program_warning: Option<String>) -> Value {
    json!({"loaded":false,"media_pending":true,"scene_applied":false,"playback":"paused","program_warning":program_warning})
}

fn missing_controller_programs(
    timeline: &crate::four_d::models::Timeline,
    catalog: Option<&crate::four_d::controller::HardwareCapabilities>,
) -> Option<String> {
    use crate::four_d::models::{ControllerEffectLane, controller_effect_timeline_track_key};
    use std::collections::BTreeSet;

    let placed: BTreeSet<_> = timeline.instances.iter().map(|cue| cue.effect_id).collect();
    let mut sequences = BTreeSet::new();
    let mut lighting = BTreeSet::new();
    for effect in timeline.templates.iter().filter(|effect| placed.contains(&effect.id)) {
        let lane = effect.controller_lane.unwrap_or(ControllerEffectLane::Sequence);
        if let Some(sequence) = &effect.controller_macro
            && timeline.track_state(&controller_effect_timeline_track_key(lane)).linked
        {
            sequences.insert(sequence.id);
        }
        if let Some(strip) = &effect.controller_strip_effect
            && timeline.track_state(&controller_effect_timeline_track_key(ControllerEffectLane::Lighting)).linked
        {
            lighting.insert(strip.id.as_str());
        }
    }
    if sequences.is_empty() && lighting.is_empty() {
        return None;
    }
    let Some(catalog) = catalog else {
        return Some("Connect to PCController to verify this timeline’s sequence and lighting programs before playback.".into());
    };
    let known_sequences: BTreeSet<_> = catalog.macros.iter().map(|item| item.id).collect();
    let known_lighting: BTreeSet<_> = catalog.strip_effects.iter().map(|item| item.id.as_str()).collect();
    let missing_sequences = sequences.difference(&known_sequences);
    let missing_lighting = lighting.difference(&known_lighting);
    let missing: Vec<_> = missing_sequences.map(|id| format!("sequence {id}"))
        .chain(missing_lighting.map(|id| format!("lighting effect {id}"))).collect();
    if missing.is_empty() {
        return None;
    }
    let shown = missing.iter().take(5).cloned().collect::<Vec<_>>().join(", ");
    let more = if missing.len() > 5 { format!(" and {} more", missing.len() - 5) } else { String::new() };
    Some(format!("Unavailable on this PCController: {shown}{more}. Check these cue references before playback."))
}

fn milliseconds(value:f64)->u64 {if value.is_finite() { (value.max(0.0)*1000.0).round().min(MAX_TIME_MS as f64) as u64 } else {0}}
fn track(mpv:&crate::mpv::player::Player,key:&str)->String {mpv.get_property::<String>(key).ok().filter(|s|matches!(s.as_str(),"auto"|"no")||s.parse::<u32>().is_ok()).unwrap_or_else(||"auto".into())}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controller_dependencies_follow_placed_linked_cues_and_live_catalog() {
        use crate::four_d::{controller::{HardwareCapabilities, HardwareStripEffect}, models::{Effect, EffectInstance, Timeline}};

        let strip = Effect::controller_strip_effect("Cinema lights".into(), 1000, "cinema-lights".into());
        let mut timeline = Timeline::default();
        timeline.templates.push(strip.clone());
        assert_eq!(missing_controller_programs(&timeline, None), None);

        timeline.instances.push(EffectInstance::new(strip.id, 3000));
        assert!(missing_controller_programs(&timeline, None).unwrap().contains("Connect to PCController"));
        let mut catalog = HardwareCapabilities::default();
        assert!(missing_controller_programs(&timeline, Some(&catalog)).unwrap().contains("cinema-lights"));
        catalog.strip_effects.push(HardwareStripEffect { id: "cinema-lights".into(), ..Default::default() });
        assert_eq!(missing_controller_programs(&timeline, Some(&catalog)), None);
    }

    #[test]
    fn file_preparation_fails_before_an_invalid_restore_can_reach_the_session() {
        let request = Request::Stage {
            document: Document::Project(Project {
                media: Some(Part::File { path: "missing.media.json".into() }),
                ..Project::default()
            }),
            base_path: std::env::temp_dir().join(uuid::Uuid::new_v4().to_string()).join("project.json").to_string_lossy().into_owned(),
            components: None,
            discard_unsaved: false,
        };
        assert!(prepare_file_request(request).is_err());
    }

    #[test]
    fn restore_fence_requires_the_requested_decoder_seek_and_paused_media() {
        let seek = ProjectSeek { request_id: 7, target: "https://example.invalid/media.mp4".into() };
        assert!(!seek.confirmed(6, &seek.target, true));
        assert!(!seek.confirmed(7, "https://example.invalid/other.mp4", true));
        assert!(!seek.confirmed(7, &seek.target, false));
        assert!(seek.confirmed(7, &seek.target, true));
    }
}
