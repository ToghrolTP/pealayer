//! WM_COPYDATA UTF-8 JSON bridge. Copy borrowed bytes before returning from the
//! WndProc; dispatch and bounded replies on a worker, never under the UI lock.
use std::sync::{
    Arc, Mutex, OnceLock,
    mpsc::{self, SyncSender},
};
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    System::DataExchange::COPYDATASTRUCT,
    UI::WindowsAndMessaging::{
        GetWindowThreadProcessId, IsWindow, SMTO_ABORTIFHUNG, SMTO_BLOCK, SendMessageTimeoutW,
        WM_COPYDATA,
    },
};

pub const REQUEST: usize = 0x504c4a51; // PLJQ
pub const RESPONSE: usize = 0x504c4a52; // PLJR; never interpreted as a request
const LIMIT: usize = 1024 * 1024;
struct Request {
    payload: String,
    reply: isize,
    reply_pid: u32,
    receiver: isize,
}
static QUEUE: OnceLock<SyncSender<Request>> = OnceLock::new();

pub fn start(
    tx: mpsc::Sender<super::interop::InteropCommand>,
    ctx: eframe::egui::Context,
    identity: String,
) {
    let (sender, receiver) = mpsc::sync_channel::<Request>(32);
    if QUEUE.set(sender).is_err() {
        return;
    }
    let receipts = Arc::new(Mutex::new(super::interop::LaunchReceiptCache::default()));
    let session = super::windows::current_session_id().ok();
    let worker = std::thread::Builder::new()
        .name("pealayer-copydata".into())
        .spawn(move || {
            while let Ok(request) = receiver.recv() {
                let response = super::interop::dispatch_local_payload(
                    &request.payload,
                    &tx,
                    &ctx,
                    &receipts,
                    &identity,
                    session,
                );
                if request.reply == 0 {
                    continue;
                }
                let hwnd = HWND(request.reply as *mut _);
                let mut pid = 0;
                unsafe {
                    GetWindowThreadProcessId(hwnd, Some(&mut pid));
                }
                if pid != request.reply_pid || !unsafe { IsWindow(Some(hwnd)) }.as_bool() {
                    continue;
                }
                let response = if response.len() > LIMIT {
                    super::interop::format_interop_error(
                        None,
                        -32000,
                        "Response exceeds WM_COPYDATA limit; use a compact query",
                    )
                } else {
                    response
                };
                let data = COPYDATASTRUCT {
                    dwData: RESPONSE,
                    cbData: response.len() as u32,
                    lpData: response.as_ptr() as *mut _,
                };
                let mut result = 0;
                unsafe {
                    SendMessageTimeoutW(
                        hwnd,
                        WM_COPYDATA,
                        WPARAM(request.receiver as usize),
                        LPARAM(&data as *const _ as isize),
                        SMTO_ABORTIFHUNG | SMTO_BLOCK,
                        500,
                        Some(&mut result),
                    );
                }
            }
        });
    if let Err(error) = worker {
        log::error!("Start WM_COPYDATA bridge: {error}");
    }
}

/// Returning 1 means bytes were safely queued, not that a mutation has applied.
/// A nonzero sender HWND receives a JSON/RPC result with RESPONSE dwData.
pub unsafe fn receive(receiver: HWND, sender: WPARAM, data: LPARAM) -> Option<LRESULT> {
    if data.0 == 0 {
        return Some(LRESULT(0));
    }
    let data = unsafe { &*(data.0 as *const COPYDATASTRUCT) };
    if data.dwData != REQUEST {
        return None;
    }
    if data.cbData == 0 || data.cbData as usize > LIMIT || data.lpData.is_null() {
        return Some(LRESULT(0));
    }
    let bytes =
        unsafe { std::slice::from_raw_parts(data.lpData.cast::<u8>(), data.cbData as usize) };
    let Ok(payload) = std::str::from_utf8(bytes.strip_suffix(&[0]).unwrap_or(bytes)) else {
        return Some(LRESULT(0));
    };
    let reply = sender.0 as isize;
    if reply == receiver.0 as isize {
        return Some(LRESULT(0));
    }
    let mut pid = 0;
    if reply != 0 {
        unsafe {
            GetWindowThreadProcessId(HWND(reply as *mut _), Some(&mut pid));
        }
        if pid == 0 {
            return Some(LRESULT(0));
        }
        let mut reply_session = 0;
        if unsafe {
            windows::Win32::System::RemoteDesktop::ProcessIdToSessionId(pid, &mut reply_session)
        }
        .is_err()
            || super::windows::current_session_id().ok() != Some(reply_session)
        {
            return Some(LRESULT(0));
        }
    }
    let accepted = QUEUE.get().is_some_and(|queue| {
        queue
            .try_send(Request {
                payload: payload.to_owned(),
                reply,
                reply_pid: pid,
                receiver: receiver.0 as isize,
            })
            .is_ok()
    });
    Some(LRESULT(isize::from(accepted)))
}
