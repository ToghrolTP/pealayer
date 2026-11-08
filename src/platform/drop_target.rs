//! HWND-owned OLE target. The Shell owns drag imagery and the IDataObject;
//! Pealayer supplies a contextual DROPDESCRIPTION and normal egui file events.
use eframe::egui;
use std::{
    cell::RefCell,
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicIsize, Ordering},
    },
};
use windows::{
    Win32::{
        Foundation::{HWND, POINT, POINTL},
        Graphics::Gdi::ScreenToClient,
        System::{
            Com::{
                CLSCTX_INPROC_SERVER, CoCreateInstance, DVASPECT_CONTENT, FORMATETC, IDataObject,
                STGMEDIUM, STGMEDIUM_0, TYMED_HGLOBAL,
            },
            DataExchange::RegisterClipboardFormatW,
            Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock},
            Ole::{
                DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_NONE, IDropTarget, IDropTarget_Impl,
                RegisterDragDrop, ReleaseStgMedium, RevokeDragDrop,
            },
            SystemServices::MODIFIERKEYS_FLAGS,
        },
        UI::Shell::{
            CLSID_DragDropHelper, DROPDESCRIPTION, DROPIMAGE_COPY, DragQueryFileW, HDROP,
            IDropTargetHelper,
        },
    },
    core::{Ref, Result, implement},
};

static OWNER: AtomicIsize = AtomicIsize::new(0);
#[derive(Default)]
struct Pending {
    hover: Vec<PathBuf>,
    dropped: Vec<PathBuf>,
    point: Option<egui::Pos2>,
    drop_point: Option<egui::Pos2>,
}
static PENDING: Mutex<Pending> = Mutex::new(Pending {
    hover: Vec::new(),
    dropped: Vec::new(),
    point: None,
    drop_point: None,
});
thread_local! {static TARGET:RefCell<Option<IDropTarget>>=const{RefCell::new(None)};}

#[implement(IDropTarget)]
struct Target {
    hwnd: isize,
    ctx: egui::Context,
    helper: Option<IDropTargetHelper>,
    data: RefCell<Option<IDataObject>>,
    paths: RefCell<Vec<PathBuf>>,
}
impl Target {
    fn point(&self, pt: &POINTL) -> egui::Pos2 {
        let mut point = POINT { x: pt.x, y: pt.y };
        unsafe {
            let _ = ScreenToClient(HWND(self.hwnd as *mut _), &mut point);
        }
        egui::pos2(point.x as f32, point.y as f32) / self.ctx.pixels_per_point()
    }
    fn feedback(&self, pt: &POINTL, effect: *mut DROPEFFECT) -> DROPEFFECT {
        let point = self.point(pt);
        let paths = self.paths.borrow();
        let blocked = self.ctx.memory(|m| m.top_modal_layer().is_some());
        let allowed = !blocked
            && !paths.is_empty()
            && !effect.is_null()
            && unsafe { (*effect).0 & DROPEFFECT_COPY.0 != 0 };
        let value = if allowed {
            DROPEFFECT_COPY
        } else {
            DROPEFFECT_NONE
        };
        if !effect.is_null() {
            unsafe {
                *effect = value;
            }
        }
        if let Ok(mut pending) = PENDING.lock() {
            pending.hover = if allowed { paths.clone() } else { Vec::new() };
            pending.point = Some(point);
        }
        if allowed && let Some(data) = self.data.borrow().as_ref() {
            let feedback = crate::assets::drop_feedback(
                &paths,
                &crate::assets::native_destination_at(&self.ctx, point),
            );
            let _ = description(data, &feedback.message);
        }
        self.ctx.request_repaint();
        value
    }
}
impl IDropTarget_Impl for Target_Impl {
    fn DragEnter(
        &self,
        data: Ref<IDataObject>,
        _: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> Result<()> {
        *self.paths.borrow_mut() = data
            .as_ref()
            .and_then(|d| read_files(d).ok())
            .unwrap_or_default();
        *self.data.borrow_mut() = data.cloned();
        let value = self.feedback(pt, effect);
        if let (Some(helper), Some(data)) = (&self.helper, data.as_ref()) {
            unsafe {
                let _ = helper.DragEnter(
                    HWND(self.hwnd as *mut _),
                    data,
                    &POINT { x: pt.x, y: pt.y },
                    value,
                );
            }
        }
        Ok(())
    }
    fn DragOver(&self, _: MODIFIERKEYS_FLAGS, pt: &POINTL, effect: *mut DROPEFFECT) -> Result<()> {
        let value = self.feedback(pt, effect);
        if let Some(helper) = &self.helper {
            unsafe {
                let _ = helper.DragOver(&POINT { x: pt.x, y: pt.y }, value);
            }
        }
        Ok(())
    }
    fn DragLeave(&self) -> Result<()> {
        if let Some(helper) = &self.helper {
            unsafe {
                let _ = helper.DragLeave();
            }
        }
        self.data.borrow_mut().take();
        self.paths.borrow_mut().clear();
        if let Ok(mut pending) = PENDING.lock() {
            pending.hover.clear();
            pending.point = None;
        }
        self.ctx.request_repaint();
        Ok(())
    }
    fn Drop(
        &self,
        data: Ref<IDataObject>,
        _: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> Result<()> {
        let value = self.feedback(pt, effect);
        if value == DROPEFFECT_COPY
            && let Ok(mut pending) = PENDING.lock()
        {
            pending.dropped = self.paths.borrow().clone();
            pending.drop_point = Some(self.point(pt));
            pending.hover.clear();
        }
        if let (Some(helper), Some(data)) = (&self.helper, data.as_ref()) {
            unsafe {
                let _ = helper.Drop(data, &POINT { x: pt.x, y: pt.y }, value);
            }
        }
        self.data.borrow_mut().take();
        self.paths.borrow_mut().clear();
        self.ctx.request_repaint();
        Ok(())
    }
}
fn read_files(data: &IDataObject) -> Result<Vec<PathBuf>> {
    use std::os::windows::ffi::OsStringExt;
    let format = FORMATETC {
        cfFormat: 15,
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
        ..Default::default()
    };
    unsafe {
        let mut medium = data.GetData(&format)?;
        // Always release the medium returned by GetData, never DragFinish it.
        let result = (|| {
            let handle = HDROP(medium.u.hGlobal.0);
            let count = DragQueryFileW(handle, u32::MAX, None);
            if count == 0 || count as usize > crate::assets::MAX_FILES {
                return Err(windows::core::Error::from_hresult(
                    windows::Win32::Foundation::E_INVALIDARG,
                ));
            }
            let mut paths = Vec::new();
            for index in 0..count {
                let length = DragQueryFileW(handle, index, None) as usize;
                if length == 0 || length > 32767 {
                    return Err(windows::core::Error::from_hresult(
                        windows::Win32::Foundation::E_INVALIDARG,
                    ));
                }
                let mut path = vec![0u16; length + 1];
                DragQueryFileW(handle, index, Some(&mut path));
                paths.push(std::ffi::OsString::from_wide(&path[..length]).into());
            }
            Ok(paths)
        })();
        ReleaseStgMedium(&mut medium);
        result
    }
}
fn description(data: &IDataObject, message: &str) -> Result<()> {
    unsafe {
        let memory = GlobalAlloc(GMEM_MOVEABLE, std::mem::size_of::<DROPDESCRIPTION>())?;
        let ptr = GlobalLock(memory);
        if ptr.is_null() {
            let _ = windows::Win32::Foundation::GlobalFree(Some(memory));
            return Err(windows::core::Error::from_hresult(
                windows::Win32::Foundation::E_OUTOFMEMORY,
            ));
        }
        let mut text = [0u16; 260];
        for (to, from) in text
            .iter_mut()
            .take(259)
            .zip(message.replace('%', "％").encode_utf16())
        {
            *to = from;
        }
        // Packed Shell ABI: write the complete value, not references to fields.
        ptr.cast::<DROPDESCRIPTION>()
            .write_unaligned(DROPDESCRIPTION {
                r#type: DROPIMAGE_COPY,
                szMessage: text,
                szInsert: [0; 260],
            });
        let _ = GlobalUnlock(memory);
        let format = FORMATETC {
            cfFormat: RegisterClipboardFormatW(windows::core::w!("DropDescription")) as u16,
            dwAspect: DVASPECT_CONTENT.0,
            lindex: -1,
            tymed: TYMED_HGLOBAL.0 as u32,
            ..Default::default()
        };
        let mut medium = STGMEDIUM {
            tymed: TYMED_HGLOBAL.0 as u32,
            u: STGMEDIUM_0 { hGlobal: memory },
            pUnkForRelease: std::mem::ManuallyDrop::new(None),
        };
        let result = data.SetData(&format, &medium, true);
        if result.is_err() {
            ReleaseStgMedium(&mut medium);
        }
        result
    }
}
pub fn install(hwnd: isize, ctx: &egui::Context) -> Result<()> {
    if OWNER.load(Ordering::Acquire) == hwnd {
        return Ok(());
    }
    unsafe {
        let helper = CoCreateInstance(&CLSID_DragDropHelper, None, CLSCTX_INPROC_SERVER).ok();
        let target: IDropTarget = Target {
            hwnd,
            ctx: ctx.clone(),
            helper,
            data: RefCell::new(None),
            paths: RefCell::new(Vec::new()),
        }
        .into();
        // Winit's generic file target has no drop-description hook. Replace it
        // only on this main HWND and retain the normal egui ingestion contract.
        RevokeDragDrop(HWND(hwnd as *mut _))?;
        RegisterDragDrop(HWND(hwnd as *mut _), &target)?;
        TARGET.with(|slot| *slot.borrow_mut() = Some(target));
        OWNER.store(hwnd, Ordering::Release);
        Ok(())
    }
}
pub fn clear(hwnd: isize) {
    if OWNER
        .compare_exchange(hwnd, 0, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
    {
        unsafe {
            let _ = RevokeDragDrop(HWND(hwnd as *mut _));
        }
        TARGET.with(|slot| {
            slot.borrow_mut().take();
        });
        if let Ok(mut pending) = PENDING.lock() {
            *pending = Pending::default();
        }
    }
}
#[derive(Debug)]
struct File(PathBuf);
impl egui::DroppedFile for File {
    fn path(&self) -> &std::path::Path {
        &self.0
    }
    fn bytes(&self) -> std::result::Result<Vec<u8>, String> {
        std::fs::read(&self.0).map_err(|e| e.to_string())
    }
}
pub fn input(raw: &mut egui::RawInput) {
    if OWNER.load(Ordering::Acquire) == 0 {
        return;
    }
    if let Ok(mut p) = PENDING.lock() {
        raw.hovered_files = p
            .hover
            .iter()
            .map(|path| egui::HoveredFile {
                path: Some(path.clone()),
                mime: String::new(),
            })
            .collect();
        let hover_point = p.point.take();
        let point = p.drop_point.take().or(hover_point);
        if let Some(point) = point {
            raw.events.push(egui::Event::PointerMoved(point));
        }
        raw.dropped_files.extend(
            p.dropped
                .drain(..)
                .map(|p| std::sync::Arc::new(File(p)) as egui::DroppedFileHandle),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shell_data_object_reports_action_and_delivers_each_file_once() {
        use windows::Win32::{
            System::Ole::{OleInitialize, OleUninitialize},
            UI::Shell::{DROPFILES, SHCreateDataObject},
        };
        unsafe {
            OleInitialize(None).unwrap();
            struct Ole;
            impl Drop for Ole {
                fn drop(&mut self) {
                    unsafe {
                        OleUninitialize();
                    }
                }
            }
            let _ole = Ole;
            let data: IDataObject = SHCreateDataObject(None, None, None::<&IDataObject>).unwrap();
            let path = "C:\\media\\فیلم.mkv";
            let names = path.encode_utf16().chain([0, 0]).collect::<Vec<_>>();
            let size = std::mem::size_of::<DROPFILES>();
            let memory = GlobalAlloc(GMEM_MOVEABLE, size + names.len() * 2).unwrap();
            let ptr = GlobalLock(memory).cast::<u8>();
            assert!(!ptr.is_null());
            ptr.cast::<DROPFILES>().write_unaligned(DROPFILES {
                pFiles: size as u32,
                fWide: true.into(),
                ..Default::default()
            });
            std::ptr::copy_nonoverlapping(
                names.as_ptr().cast::<u8>(),
                ptr.add(size),
                names.len() * 2,
            );
            let _ = GlobalUnlock(memory);
            let format = FORMATETC {
                cfFormat: 15,
                dwAspect: DVASPECT_CONTENT.0,
                lindex: -1,
                tymed: TYMED_HGLOBAL.0 as u32,
                ..Default::default()
            };
            let medium = STGMEDIUM {
                tymed: TYMED_HGLOBAL.0 as u32,
                u: STGMEDIUM_0 { hGlobal: memory },
                pUnkForRelease: std::mem::ManuallyDrop::new(None),
            };
            data.SetData(&format, &medium, true).unwrap();
            assert_eq!(read_files(&data).unwrap(), [PathBuf::from(path)]);
            description(&data, "Play فیلم.mkv").unwrap();
            let format = FORMATETC {
                cfFormat: RegisterClipboardFormatW(windows::core::w!("DropDescription")) as u16,
                ..format
            };
            let mut desc = data.GetData(&format).unwrap();
            let ptr = GlobalLock(desc.u.hGlobal);
            let value = ptr.cast::<DROPDESCRIPTION>().read_unaligned();
            let text = value.szMessage;
            assert_eq!(
                String::from_utf16_lossy(&text[..text.iter().position(|c| *c == 0).unwrap()]),
                "Play فیلم.mkv"
            );
            let _ = GlobalUnlock(desc.u.hGlobal);
            ReleaseStgMedium(&mut desc);
            // Direct COM calls use the same production target; no desktop drag,
            // system clipboard, application window or file read is performed.
            let target: IDropTarget = Target {
                hwnd: 0,
                ctx: egui::Context::default(),
                helper: None,
                data: RefCell::new(None),
                paths: RefCell::new(Vec::new()),
            }
            .into();
            let mut effect = DROPEFFECT_COPY;
            let point = POINTL { x: 20, y: 30 };
            target
                .DragEnter(&data, MODIFIERKEYS_FLAGS(0), point, &mut effect)
                .unwrap();
            assert_eq!(effect, DROPEFFECT_COPY);
            effect = DROPEFFECT_COPY;
            target
                .Drop(&data, MODIFIERKEYS_FLAGS(0), point, &mut effect)
                .unwrap();
            target.DragLeave().unwrap();
            OWNER.store(-1, Ordering::Release);
            let mut input = egui::RawInput::default();
            super::input(&mut input);
            assert_eq!(input.dropped_files.len(), 1);
            assert!(
                input
                    .events
                    .iter()
                    .any(|e| matches!(e,egui::Event::PointerMoved(p) if p.x==20.0 && p.y==30.0))
            );
            assert_eq!(input.dropped_files[0].path(), std::path::Path::new(path));
            let mut again = egui::RawInput::default();
            super::input(&mut again);
            assert!(again.dropped_files.is_empty());
            assert!(again.events.is_empty());
            OWNER.store(0, Ordering::Release);
            target.DragLeave().unwrap();
        }
    }
}
