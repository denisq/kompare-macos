//! Files handed to us by Finder / `open` arrive as Apple Events (`application:openFiles:`),
//! not as command-line arguments. winit's application delegate doesn't implement that selector,
//! so we add an implementation to NSObject at startup; the delegate inherits it.

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

pub static PENDING: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
pub static CTX: OnceLock<egui::Context> = OnceLock::new();

pub fn take_pending() -> Vec<PathBuf> {
    std::mem::take(&mut *PENDING.lock().unwrap())
}

pub fn push_pending(paths: Vec<PathBuf>) {
    PENDING.lock().unwrap().extend(paths);
    if let Some(ctx) = CTX.get() {
        ctx.request_repaint();
    }
}

#[cfg(target_os = "macos")]
pub fn install_open_files_hook() {
    use objc2::runtime::{AnyClass, AnyObject, Imp, Sel};
    use objc2::{msg_send, sel};
    use std::ffi::{c_char, CStr};

    unsafe extern "C-unwind" fn open_files(
        _this: *mut AnyObject,
        _cmd: Sel,
        app: *mut AnyObject,
        files: *mut AnyObject,
    ) {
        let mut paths = Vec::new();
        unsafe {
            let count: usize = msg_send![files, count];
            for i in 0..count {
                let s: *mut AnyObject = msg_send![files, objectAtIndex: i];
                let c: *const c_char = msg_send![s, UTF8String];
                if !c.is_null() {
                    paths.push(PathBuf::from(CStr::from_ptr(c).to_string_lossy().into_owned()));
                }
            }
            // NSApplicationDelegateReplySuccess == 0
            let _: () = msg_send![app, replyToOpenOrPrint: 0usize];
        }
        push_pending(paths);
    }

    unsafe {
        let Some(cls) = AnyClass::get(c"NSObject") else { return };
        let imp: Imp = std::mem::transmute(
            open_files as unsafe extern "C-unwind" fn(*mut AnyObject, Sel, *mut AnyObject, *mut AnyObject),
        );
        objc2::ffi::class_addMethod(
            cls as *const AnyClass as *mut AnyClass,
            sel!(application:openFiles:),
            imp,
            c"v@:@@".as_ptr(),
        );
    }
}

#[cfg(not(target_os = "macos"))]
pub fn install_open_files_hook() {}
