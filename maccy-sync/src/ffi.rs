use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::sync::Arc;

use crate::error::ErrorCode;
use crate::network::NetworkManager;
use crate::state::{SharedState, SyncCommand, SyncState};

pub type MaccySync = Arc<std::sync::Mutex<SyncState>>;

fn c_str_to_string(s: *const c_char) -> Option<String> {
    if s.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(s).to_str().ok().map(String::from) }
}

#[unsafe(no_mangle)]
pub extern "C" fn maccy_sync_create(
    device_name: *const c_char,
    device_id: *const c_char,
) -> *mut MaccySync {
    let name = c_str_to_string(device_name).unwrap_or_default();
    let id = c_str_to_string(device_id).unwrap_or_default();

    match SyncState::new(&name, &id) {
        Ok(state) => Arc::into_raw(Arc::new(std::sync::Mutex::new(state))) as *mut MaccySync,
        Err(_) => ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn maccy_sync_destroy(sync: *mut MaccySync) {
    if !sync.is_null() {
        unsafe {
            let _ = Arc::from_raw(sync);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn maccy_sync_start(sync: *mut MaccySync) -> i32 {
    if sync.is_null() {
        return ErrorCode::InvalidArg as i32;
    }

    let shared: SharedState = unsafe { Arc::from_raw(sync as *const std::sync::Mutex<SyncState>) };

    let local_key = libp2p::identity::Keypair::generate_ed25519();

    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
    {
        let mut guard = shared.lock().unwrap();
        guard.command_tx = command_tx;
    }

    let mut manager = match NetworkManager::new(command_rx, shared.clone(), local_key) {
        Ok(m) => m,
        Err(e) => {
            std::mem::forget(shared);
            return e as i32;
        }
    };

    let rt = {
        let guard = shared.lock().unwrap();
        guard.runtime.handle().clone()
    };

    std::thread::spawn(move || {
        rt.block_on(async move {
            manager.run().await;
        });
    });

    std::mem::forget(shared);
    ErrorCode::Ok as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn maccy_sync_stop(sync: *mut MaccySync) -> i32 {
    if sync.is_null() {
        return ErrorCode::InvalidArg as i32;
    }

    let state = unsafe { &*sync };
    let guard = state.lock().unwrap();
    let _ = guard.command_tx.send(SyncCommand::Shutdown);
    ErrorCode::Ok as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn maccy_sync_on_peer_discovered(
    sync: *mut MaccySync,
    cb: Option<extern "C" fn(peer_id: *const c_char, display_name: *const c_char, addresses: *const c_char)>,
) {
    if sync.is_null() || cb.is_none() {
        return;
    }
    let cb = cb.unwrap();
    let state = unsafe { &*sync };
    let guard = state.lock().unwrap();
    *guard.on_peer_discovered.lock() = Some(Box::new(move |info: crate::types::PeerInfo| {
        let peer_id = CString::new(info.peer_id).unwrap_or_default();
        let name = CString::new(info.display_name).unwrap_or_default();
        let addrs = CString::new(info.addresses.join(",")).unwrap_or_default();
        cb(peer_id.as_ptr(), name.as_ptr(), addrs.as_ptr());
    }));
}

#[unsafe(no_mangle)]
pub extern "C" fn maccy_sync_on_peer_lost(
    sync: *mut MaccySync,
    cb: Option<extern "C" fn(peer_id: *const c_char)>,
) {
    if sync.is_null() || cb.is_none() {
        return;
    }
    let cb = cb.unwrap();
    let state = unsafe { &*sync };
    let guard = state.lock().unwrap();
    *guard.on_peer_lost.lock() = Some(Box::new(move |peer_id: String| {
        let c_peer_id = CString::new(peer_id).unwrap_or_default();
        cb(c_peer_id.as_ptr());
    }));
}

#[unsafe(no_mangle)]
pub extern "C" fn maccy_sync_on_pairing_request(
    sync: *mut MaccySync,
    cb: Option<extern "C" fn(peer_id: *const c_char, display_name: *const c_char, pin: *const c_char)>,
) {
    if sync.is_null() || cb.is_none() {
        return;
    }
    let cb = cb.unwrap();
    let state = unsafe { &*sync };
    let guard = state.lock().unwrap();
    *guard.on_pairing_request.lock() = Some(Box::new(move |(peer_id, display_name, pin): (String, String, String)| {
        let c_peer_id = CString::new(peer_id).unwrap_or_default();
        let c_name = CString::new(display_name).unwrap_or_default();
        let c_pin = CString::new(pin).unwrap_or_default();
        cb(c_peer_id.as_ptr(), c_name.as_ptr(), c_pin.as_ptr());
    }));
}

#[unsafe(no_mangle)]
pub extern "C" fn maccy_sync_on_pairing_complete(
    sync: *mut MaccySync,
    cb: Option<extern "C" fn(peer_id: *const c_char, success: bool)>,
) {
    if sync.is_null() || cb.is_none() {
        return;
    }
    let cb = cb.unwrap();
    let state = unsafe { &*sync };
    let guard = state.lock().unwrap();
    *guard.on_pairing_complete.lock() = Some(Box::new(move |(peer_id, success): (String, bool)| {
        let c_peer_id = CString::new(peer_id).unwrap_or_default();
        cb(c_peer_id.as_ptr(), success);
    }));
}

#[unsafe(no_mangle)]
pub extern "C" fn maccy_sync_on_sync_item_received(
    sync: *mut MaccySync,
    cb: Option<extern "C" fn(item_json: *const c_char)>,
) {
    if sync.is_null() || cb.is_none() {
        return;
    }
    let cb = cb.unwrap();
    let state = unsafe { &*sync };
    let guard = state.lock().unwrap();
    *guard.on_item_received.lock() = Some(Box::new(move |json: String| {
        let c_json = CString::new(json).unwrap_or_default();
        cb(c_json.as_ptr());
    }));
}

#[unsafe(no_mangle)]
pub extern "C" fn maccy_sync_on_sync_item_deleted(
    sync: *mut MaccySync,
    cb: Option<extern "C" fn(item_id: *const c_char)>,
) {
    if sync.is_null() || cb.is_none() {
        return;
    }
    let cb = cb.unwrap();
    let state = unsafe { &*sync };
    let guard = state.lock().unwrap();
    *guard.on_item_deleted.lock() = Some(Box::new(move |id: String| {
        let c_id = CString::new(id).unwrap_or_default();
        cb(c_id.as_ptr());
    }));
}

#[unsafe(no_mangle)]
pub extern "C" fn maccy_sync_on_sync_item_updated(
    sync: *mut MaccySync,
    cb: Option<extern "C" fn(item_json: *const c_char)>,
) {
    if sync.is_null() || cb.is_none() {
        return;
    }
    let cb = cb.unwrap();
    let state = unsafe { &*sync };
    let guard = state.lock().unwrap();
    *guard.on_item_updated.lock() = Some(Box::new(move |json: String| {
        let c_json = CString::new(json).unwrap_or_default();
        cb(c_json.as_ptr());
    }));
}

#[unsafe(no_mangle)]
pub extern "C" fn maccy_sync_on_error(
    sync: *mut MaccySync,
    cb: Option<extern "C" fn(code: i32, message: *const c_char)>,
) {
    if sync.is_null() || cb.is_none() {
        return;
    }
    let cb = cb.unwrap();
    let state = unsafe { &*sync };
    let guard = state.lock().unwrap();
    *guard.on_error.lock() = Some(Box::new(move |(code, msg): (i32, String)| {
        let c_msg = CString::new(msg).unwrap_or_default();
        cb(code, c_msg.as_ptr());
    }));
}

fn send_command(sync: *mut MaccySync, cmd: SyncCommand) -> i32 {
    if sync.is_null() {
        return ErrorCode::InvalidArg as i32;
    }
    let state = unsafe { &*sync };
    let guard = state.lock().unwrap();
    match guard.command_tx.send(cmd) {
        Ok(()) => ErrorCode::Ok as i32,
        Err(_) => ErrorCode::NotRunning as i32,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn maccy_sync_start_discovery(sync: *mut MaccySync) -> i32 {
    send_command(sync, SyncCommand::StartDiscovery)
}

#[unsafe(no_mangle)]
pub extern "C" fn maccy_sync_stop_discovery(sync: *mut MaccySync) -> i32 {
    send_command(sync, SyncCommand::StopDiscovery)
}

#[unsafe(no_mangle)]
pub extern "C" fn maccy_sync_request_pairing(
    sync: *mut MaccySync,
    peer_id: *const c_char,
) -> i32 {
    let pid = match c_str_to_string(peer_id) {
        Some(s) => s,
        None => return ErrorCode::InvalidArg as i32,
    };
    send_command(sync, SyncCommand::RequestPairing { peer_id: pid })
}

#[unsafe(no_mangle)]
pub extern "C" fn maccy_sync_accept_pairing(
    sync: *mut MaccySync,
    peer_id: *const c_char,
    pin: *const c_char,
) -> i32 {
    let pid = match c_str_to_string(peer_id) {
        Some(s) => s,
        None => return ErrorCode::InvalidArg as i32,
    };
    let p = match c_str_to_string(pin) {
        Some(s) => s,
        None => return ErrorCode::InvalidArg as i32,
    };
    send_command(sync, SyncCommand::AcceptPairing { peer_id: pid, pin: p })
}

#[unsafe(no_mangle)]
pub extern "C" fn maccy_sync_reject_pairing(
    sync: *mut MaccySync,
    peer_id: *const c_char,
) -> i32 {
    let pid = match c_str_to_string(peer_id) {
        Some(s) => s,
        None => return ErrorCode::InvalidArg as i32,
    };
    send_command(sync, SyncCommand::RejectPairing { peer_id: pid })
}

#[unsafe(no_mangle)]
pub extern "C" fn maccy_sync_broadcast_item(
    sync: *mut MaccySync,
    item_json: *const c_char,
) -> i32 {
    let json = match c_str_to_string(item_json) {
        Some(s) => s,
        None => return ErrorCode::InvalidArg as i32,
    };
    send_command(sync, SyncCommand::BroadcastItem { item_json: json })
}

#[unsafe(no_mangle)]
pub extern "C" fn maccy_sync_broadcast_deletion(
    sync: *mut MaccySync,
    item_id: *const c_char,
) -> i32 {
    let id = match c_str_to_string(item_id) {
        Some(s) => s,
        None => return ErrorCode::InvalidArg as i32,
    };
    send_command(sync, SyncCommand::BroadcastDeletion { item_id: id })
}

#[unsafe(no_mangle)]
pub extern "C" fn maccy_sync_broadcast_update(
    sync: *mut MaccySync,
    item_json: *const c_char,
) -> i32 {
    let json = match c_str_to_string(item_json) {
        Some(s) => s,
        None => return ErrorCode::InvalidArg as i32,
    };
    send_command(sync, SyncCommand::BroadcastUpdate { item_json: json })
}

#[unsafe(no_mangle)]
pub extern "C" fn maccy_sync_add_peer_address(
    sync: *mut MaccySync,
    peer_id: *const c_char,
    address: *const c_char,
) -> i32 {
    let pid = match c_str_to_string(peer_id) {
        Some(s) => s,
        None => return ErrorCode::InvalidArg as i32,
    };
    let addr = match c_str_to_string(address) {
        Some(s) => s,
        None => return ErrorCode::InvalidArg as i32,
    };
    send_command(sync, SyncCommand::AddPeerAddress {
        peer_id: pid,
        address: addr,
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn maccy_sync_unpair(
    sync: *mut MaccySync,
    peer_id: *const c_char,
) -> i32 {
    let pid = match c_str_to_string(peer_id) {
        Some(s) => s,
        None => return ErrorCode::InvalidArg as i32,
    };
    send_command(sync, SyncCommand::Unpair { peer_id: pid })
}

#[unsafe(no_mangle)]
pub extern "C" fn maccy_sync_get_paired_peers(sync: *mut MaccySync) -> *mut c_char {
    if sync.is_null() {
        return ptr::null_mut();
    }
    let c_str = CString::new("[]").unwrap();
    c_str.into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn maccy_sync_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn maccy_sync_is_running(sync: *mut MaccySync) -> bool {
    if sync.is_null() {
        return false;
    }
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn maccy_sync_get_status(sync: *mut MaccySync) -> *mut c_char {
    if sync.is_null() {
        return ptr::null_mut();
    }
    let status = r#"{"running":false}"#;
    let c_str = CString::new(status).unwrap();
    c_str.into_raw()
}
