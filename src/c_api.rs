use std::{collections::HashMap, ffi::{CStr, CString}, sync::{Mutex, OnceLock}};

use rdxusb_protocol::RdxUsbPacket;

use crate::event_loop::{self, EventLoopError};

fn to_optional_string(cs: *const i8) -> Option<String> {
    if cs == core::ptr::null() {
        None
    } else { 
        unsafe { Some(CStr::from_ptr(cs).to_string_lossy().to_string()) } 
    }
}

/// RdxUsb will attempt to keep a connection to a device matching the vid/pid/serial number pair indefinitely.
/// 
/// serial_number MUST be valid utf-8 if not a null pointer!!!
/// passing in not-utf8 is Undefined Behavior.
pub extern "C" fn rdxusb_open_device(vid: u16, pid: u16, serial_number: *const i8, close_on_dc: bool) -> i32 {
    let serial_number = to_optional_string(serial_number);
    event_loop::open_device(vid, pid, serial_number, close_on_dc).unwrap_or_else(|e| e as i32)
}


pub extern "C" fn rdxusb_force_scan_devices() -> i32 {
    let Ok(event_loop) = event_loop::try_acquire_event_loop() else { return EventLoopError::ERR_EVENT_LOOP_CRASHED; };
    match event_loop::force_scan_devices(event_loop) {
        Ok(_) => 0,
        Err(e) => e as i32,
    }
}

pub extern "C" fn rdxusb_read_packets(handle_id: i32, channel: u8, packets: *mut RdxUsbPacket, max_packets: u64, packets_read: *mut u64) -> i32 {
    let packets = unsafe { core::slice::from_raw_parts_mut(packets, max_packets as usize) };
    match event_loop::read_packets(handle_id, channel, packets) {
        Ok(w) => {
            unsafe { *packets_read = w as u64; }
            0
        }
        Err(e) => { e as i32 }
    }
}

pub extern "C" fn rdxusb_write_packets(handle_id: i32, packets: *const RdxUsbPacket, packets_len: u64, packets_written: *mut u64) -> i32 {
    let packets = unsafe { core::slice::from_raw_parts(packets, packets_len as usize) };
    match event_loop::write_packets(handle_id, packets) {
        Ok(w) => {
            unsafe { *packets_written = w as u64; }
            0
        }
        Err(e) => { e as i32 }
    }
}

pub extern "C" fn rdxusb_close_device(handle_id: i32) -> i32 {
    event_loop::close_device(handle_id).map_or_else(|e| e as i32, |_| 0)
}

pub extern "C" fn rdxusb_close_all_devices() -> i32 {
    event_loop::close_all_devices().map_or_else(|e| e as i32, |_| 0)
}

// Device Iterators --------

struct DeviceInfos {
    info_map: HashMap<u64, Vec<nusb::DeviceInfo>>,
    next_idx: u64,
}
impl DeviceInfos {
    pub fn new() -> Self {
        Self { info_map: HashMap::new(), next_idx: 0 }
    }
    pub fn allocate_idx_and_insert(&mut self, devices: Vec<nusb::DeviceInfo>) -> u64 {
        let idx = self.next_idx;
        self.info_map.insert(idx, devices);
        self.next_idx += 1;
        idx
    }

    pub fn free_idx(&mut self, idx: u64) {
        self.info_map.remove(&idx);
    }
}

static DEVICE_INFOS: Mutex<OnceLock<DeviceInfos>> = Mutex::new(OnceLock::new());

#[repr(C)]
pub struct RdxUsbDeviceEntry {
    serial: [u8; 256],
    manufacturer: [u8; 256],
    product_str: [u8; 256],
    vid: u16,
    pid: u16,
    bus_number: u8,
    device_address: u8,
}

fn strncpy_into_buf(s: &CStr, dest: &mut [u8]) {
    let max_len = dest.len() - 1;
    let full_buf = s.to_bytes_with_nul();
    let copy_buf = &full_buf[..full_buf.len().min(max_len)];
    dest[..copy_buf.len()].copy_from_slice(copy_buf);
    dest[max_len] = 0;
}

/// if you pass in null pointers your program explodes. don't do that.
pub extern "C" fn rdxusb_new_device_iterator(iter_id: *mut u64, n_devices: *mut u64) -> i32 {
    DEVICE_INFOS.lock().unwrap().get_or_init(DeviceInfos::new);
    let Ok(mut info_lock) = DEVICE_INFOS.lock() else { return EventLoopError::ERR_EVENT_LOOP_CRASHED; };
    let infos = info_lock.get_mut().unwrap();
    let Ok(device_iter) = nusb::list_devices() else { return EventLoopError::ERR_CANNOT_LIST_DEVICES; };
    let devices: Vec<nusb::DeviceInfo> = device_iter.collect();
    let devices_count = devices.len() as u64;
    let idx = infos.allocate_idx_and_insert(devices);
    unsafe {
        *iter_id = idx;
        *n_devices = devices_count;
    }
    0
}

/// passing in a null pointer is your fault. idiot.
pub extern "C" fn rdxusb_get_device_in_iterator(iter_id: u64, device_idx: u64, device_entry: *mut RdxUsbDeviceEntry) -> i32 {
    DEVICE_INFOS.lock().unwrap().get_or_init(DeviceInfos::new);
    let Ok(mut info_lock) = DEVICE_INFOS.lock() else { return EventLoopError::ERR_EVENT_LOOP_CRASHED; };
    let infos = info_lock.get_mut().unwrap();

    let Some(device_infos) = infos.info_map.get(&iter_id) else { return EventLoopError::ERR_DEVICE_ITER_INVALID; };
    let device_idx = device_idx as usize;
    if device_idx >= device_infos.len() { return EventLoopError::ERR_DEVICE_ITER_IDX_OUT_OF_RANGE; }
    let device_ent = &device_infos[device_idx];

    let device_entry = unsafe { &mut *device_entry };

    let serial_str = CString::new(device_ent.serial_number().unwrap_or("")).unwrap_or(c"".into());
    let mfg_str = CString::new(device_ent.manufacturer_string().unwrap_or("")).unwrap_or(c"".into());
    let prod_str = CString::new(device_ent.product_string().unwrap_or("")).unwrap_or(c"".into());
    strncpy_into_buf(serial_str.as_c_str(), &mut device_entry.serial);
    strncpy_into_buf(mfg_str.as_c_str(), &mut device_entry.manufacturer);
    strncpy_into_buf(prod_str.as_c_str(), &mut device_entry.product_str);

    device_entry.vid = device_ent.vendor_id();
    device_entry.pid = device_ent.product_id();
    device_entry.bus_number = device_ent.bus_number();
    device_entry.device_address = device_ent.device_address();
    0
}

pub extern "C" fn rdxusb_free_device_iterator(iter_id: u64) -> i32 {
    DEVICE_INFOS.lock().unwrap().get_or_init(DeviceInfos::new);
    let Ok(mut info_lock) = DEVICE_INFOS.lock() else { return EventLoopError::ERR_EVENT_LOOP_CRASHED; };
    let infos = info_lock.get_mut().unwrap();
    infos.free_idx(iter_id);
    0
}