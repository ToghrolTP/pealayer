//! Fresh, process-local draft inspection. No draft content leaves the UI thread.
use std::collections::BTreeMap;
use std::sync::{Mutex, atomic::{AtomicU64, Ordering}, mpsc};
use std::time::Duration;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum State { Clean, Dirty, Busy, Unknown }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item { pub domain: String, pub state: State }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Inspection {
    pub process_id: u32,
    pub git_commit: String,
    pub scope: String,
    pub state: State,
    pub safe_to_replace_native: bool,
    pub items: Vec<Item>,
}

impl Inspection {
    pub fn new(items: Vec<Item>) -> Self {
        let state=if items.iter().any(|item|item.state==State::Busy) {State::Busy}
            else if items.iter().any(|item|item.state==State::Dirty) {State::Dirty}
            else if items.iter().any(|item|item.state==State::Unknown) {State::Unknown}
            else {State::Clean};
        Self {process_id:std::process::id(),git_commit:env!("PEALAYER_GIT_COMMIT").into(),
            scope:"local native process; external browser/mobile drafts are not inspected".into(),
            safe_to_replace_native:state==State::Clean,state,items}
    }
}

static NEXT: AtomicU64=AtomicU64::new(1);
static PENDING: Mutex<BTreeMap<u64,mpsc::Sender<Inspection>>>=Mutex::new(BTreeMap::new());

/// An unresponsive/minimized UI is unknown, never silently considered clean.
pub(crate) fn inspect(commands:&mpsc::Sender<crate::platform::interop::InteropCommand>,ctx:&eframe::egui::Context) -> Result<Inspection,String> {
    inspect_with_timeout(commands,ctx,Duration::from_secs(3))
}

fn inspect_with_timeout(commands:&mpsc::Sender<crate::platform::interop::InteropCommand>,ctx:&eframe::egui::Context,timeout:Duration) -> Result<Inspection,String> {
    let id=NEXT.fetch_add(1,Ordering::Relaxed);
    let (tx,rx)=mpsc::channel();
    {
        let mut pending=PENDING.lock().map_err(|_|"Authoring inspection unavailable")?;
        if pending.len()>=8 {return Err("Authoring inspection queue is busy".into());}
        pending.insert(id,tx);
    }
    let result=if commands.send(crate::platform::interop::InteropCommand::InspectAuthoring {request_id:id}).is_err() {
        Err("Native authoring process is unavailable".into())
    } else {
        ctx.request_repaint();
        rx.recv_timeout(timeout).map_err(|_|"Native authoring inspection timed out; draft state is unknown".into())
    };
    if let Ok(mut pending)=PENDING.lock() {pending.remove(&id);}
    result
}

pub(crate) fn requested(id:u64) -> bool {PENDING.lock().is_ok_and(|pending|pending.contains_key(&id))}
pub(crate) fn complete(id:u64,inspection:Inspection) {
    if let Some(reply)=PENDING.lock().ok().and_then(|mut pending|pending.remove(&id)) {let _=reply.send(inspection);}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn readiness_never_confuses_unknown_busy_or_dirty_with_clean() {
        for state in [State::Clean,State::Dirty,State::Busy,State::Unknown] {
            let value=Inspection::new(vec![Item {domain:"rf".into(),state:state.clone()}]);
            assert_eq!(value.safe_to_replace_native,state==State::Clean);assert_eq!(value.state,state);
            let json=serde_json::to_string(&value).unwrap();
            assert!(!json.contains("draft_content"));
        }
    }
    #[test]
    fn internal_query_cannot_be_injected_or_relayed_as_a_session_command() {
        let command=crate::platform::interop::InteropCommand::InspectAuthoring {request_id:1};
        assert!(command.is_process_local());
        assert!(serde_json::from_str::<crate::platform::interop::InteropCommand>(r#"{"command":"inspect_authoring","request_id":1}"#).is_err());
    }
    #[test]
    fn inspection_is_correlated_read_only_and_timeout_is_not_clean() {
        let context=eframe::egui::Context::default();let (tx,rx)=mpsc::channel();
        let worker=std::thread::spawn(move || {
            let command=rx.recv().unwrap();
            let crate::platform::interop::InteropCommand::InspectAuthoring {request_id}=command else {panic!("inspection must never issue a save/quit/hardware command")};
            complete(request_id,Inspection::new(vec![Item {domain:"rf".into(),state:State::Dirty}]));
        });
        assert_eq!(inspect(&tx,&context).unwrap().state,State::Dirty);worker.join().unwrap();
        let (tx,_rx)=mpsc::channel();
        assert!(inspect_with_timeout(&tx,&context,Duration::from_millis(10)).unwrap_err().contains("unknown"));
    }
    #[test]
    fn actual_native_drafts_and_pending_saves_are_detected_without_saving_or_sending_commands() {
        let _guard=crate::app::tests::lock_app_tests();
        let mut app=crate::app::PealayerApp::default();
        app.engine_handle=crate::four_d::engine::memory_only_test_engine();
        let ctx=eframe::egui::Context::default();
        assert!(app.inspect_authoring(&ctx).safe_to_replace_native);
        app.show_effect_library_editor=true;app.effect_library_draft.is_new=true;
        app.effect_library_draft.name="Private unsaved title".into();
        let before=app.effect_library_draft.clone();
        let status=app.inspect_authoring(&ctx);
        assert_eq!(status.state,State::Dirty);
        assert!(!serde_json::to_string(&status).unwrap().contains("Private unsaved title"));
        assert_eq!(app.effect_library_draft,before);
        app.hardware_effect_authoring.pending_operation=Some("fixture".into());
        assert_eq!(app.inspect_authoring(&ctx).state,State::Busy);
        app.hardware_effect_authoring.pending_operation=None;app.show_effect_library_editor=false;
        app.effect_group_draft=Some(crate::app::ControllerEffectGroupDraft {original_name:String::new(),name:"unsaved".into(),icon:String::new()});
        assert_eq!(app.inspect_authoring(&ctx).state,State::Unknown);
    }
}
