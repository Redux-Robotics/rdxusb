#![allow(unused)]

use std::{cell::OnceCell, collections::HashMap, ops::{Deref, DerefMut}, sync::{Arc, Mutex, MutexGuard}};
use futures_util::stream::StreamExt;
use nusb::{DeviceId, DeviceInfo};
use rdxusb_protocol::RdxUsbPacket;
use tokio::runtime::Runtime;

use crate::host::{RdxUsbFsChannel, RdxUsbFsHost, RdxUsbFsWriter, RdxUsbHostError};

/*

Architecture:

Pollers:
* They own an RdxUsbHost
* They are responsible for polling the RdxUsbHost until a fault happens


*/



#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum EventLoopError {
    None = 0,
    EventLoopCrashed = -100,
    CannotListDevices = -101,
    DeviceIterInvalid = -102,
    DeviceNotOpened = -200,
    DeviceNotConnected = -201,
    ChannelOutOfRange = -202,
}

impl EventLoopError {
    pub const ERR_EVENT_LOOP_CRASHED: i32 = -100;
    pub const ERR_CANNOT_LIST_DEVICES: i32 = -101;
    pub const ERR_DEVICE_ITER_INVALID: i32 = -102;
    pub const ERR_DEVICE_ITER_IDX_OUT_OF_RANGE: i32 = -103;
    pub const ERR_DEVICE_NOT_OPENED: i32 = -200;
    pub const ERR_DEVICE_NOT_CONNECTED: i32 = -201;
    pub const ERR_CHANNEL_OUT_OF_RANGE: i32 = -202;
}

impl From<EventLoopError> for i32 {
    fn from(value: EventLoopError) -> Self {
        value as i32
    }
}


pub enum DeviceIOError {
    ChannelOutOfRange,
    NoData,
}

pub enum DeviceChannels {
    FsDevice(Vec<RdxUsbFsChannel>),
}

pub enum Writer {
    FsDevice(RdxUsbFsWriter),
}

impl DeviceChannels {}

pub struct OpenDevice {
    pub channels: DeviceChannels,
    pub writer: Writer,
    pub device_id: DeviceId,
    pub protocol: u8,
}

impl OpenDevice {
    pub fn try_read(&mut self, channel_idx: u8) -> Result<RdxUsbPacket, DeviceIOError> {
        match &mut self.channels {
            DeviceChannels::FsDevice(vec) => {
                if vec.len() <= channel_idx as usize { return Err(DeviceIOError::ChannelOutOfRange); }
                match vec[channel_idx as usize].try_read() {
                    Some(p) => Ok(p.into()),
                    None => Err(DeviceIOError::NoData)
                }
            }
        }
    }

    pub async fn read(&mut self, channel_idx: u8) -> Result<RdxUsbPacket, RdxUsbHostError> {
        match &mut self.channels {
            DeviceChannels::FsDevice(vec) => {
                if vec.len() <= channel_idx as usize { return Err(RdxUsbHostError::NoInterface); }
                Ok(vec[channel_idx as usize].read().await?.into())
            }
        }
    }

    pub fn try_write(&mut self, packet: &RdxUsbPacket) -> Result<(), RdxUsbPacket> {
        match &mut self.writer {
            Writer::FsDevice(writer) => {
                match writer.try_send(packet.clone().try_into()?) {
                    Some(s) => Err(s.into()),
                    None => Ok(())
                }
            }
        }
    }

    pub async fn write(&mut self, packet: RdxUsbPacket)  -> Result<(), RdxUsbPacket> {
        match &mut self.writer {
            Writer::FsDevice(writer) => {
                match writer.send(packet.try_into()?).await {
                    Ok(_) => Ok(()),
                    Err(p) => Err(p.into())
                }
            }
        }
    }
}

#[allow(unused)]
pub struct Device {
    pub vid: u16,
    pub pid: u16,
    pub serial_number: Option<String>,
    pub handle: Option<OpenDevice>,
    pub poller_handle: tokio::task::JoinHandle<()>,
    pub device_info_out: tokio::sync::watch::Sender<Option<DeviceInfo>>,
    pub shutdown: Arc<tokio::sync::Notify>,
}

impl Device {
    pub fn matches(&self, vid: u16, pid: u16, serial_number: Option<&str>) -> bool {
        self.vid == vid && self.pid == pid && (match &self.serial_number {
            Some(s) => match serial_number {
                Some(s2) => s2 == s.as_str(),
                None => false
            }
            None => true
        })
    }
    pub fn matches_device_info(&self, info: &DeviceInfo) -> bool {
        self.vid == info.vendor_id() && self.pid == info.product_id() && (match &self.serial_number {
            Some(s) => info.serial_number().map_or(false, |ins| s.as_str() == ins),
            None => true,
        })
    }
}


pub struct EventLoop {
    pub devices: HashMap<i32, Device>,
    pub next_handle: i32,
    pub rt: Runtime,
}

impl EventLoop {
    pub fn new() -> Self {
        let rt = Runtime::new().expect("Unable to create tokio runtime");

        // Enter the runtime so that `tokio::spawn` is available immediately.
        let _enter = rt.enter();
        //rt.spawn(async move { run(state_async).await.unwrap(); });
        Self {
            devices: HashMap::new(),
            next_handle: 0i32,
            rt,
        }

    }

    pub fn update_open_device(&mut self, id: i32, device: OpenDevice) {
        self.devices.get_mut(&id).unwrap().handle.replace(device);

    }

    pub fn remove_open_device(&mut self, id: i32) {
        self.devices.get_mut(&id).unwrap().handle.take();
    }

    pub fn acquire_open_device(&mut self, id: i32) -> Result<&mut OpenDevice, EventLoopError> {
        let Some(device) = self.devices.get_mut(&id) else { return Err(EventLoopError::DeviceNotOpened); };
        let Some(open_device) = device.handle.as_mut() else { return Err(EventLoopError::DeviceNotConnected); };
        Ok(open_device)
    }

}

static EVENT_LOOP: Mutex<OnceCell<EventLoop>> = Mutex::new(OnceCell::new());
pub struct EventLoopGuard<'a>(MutexGuard<'a, OnceCell<EventLoop>>);
impl<'a> Deref for EventLoopGuard<'a> {
    type Target = EventLoop;
    fn deref(&self) -> &Self::Target {
        self.0.get().unwrap()
    }
}

impl<'a> DerefMut for EventLoopGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.get_mut().unwrap()
    }
}

pub fn acquire_event_loop<'a>() -> EventLoopGuard<'a> {
    let event_loop_lock = EVENT_LOOP.lock().unwrap();
    event_loop_lock.get_or_init(EventLoop::new);
    EventLoopGuard(event_loop_lock)
}

pub fn try_acquire_event_loop<'a>() -> Result<EventLoopGuard<'a>, EventLoopError> {
    let event_loop_lock = EVENT_LOOP.lock().map_err(|_e| EventLoopError::EventLoopCrashed)?;
    event_loop_lock.get_or_init(EventLoop::new);
    Ok(EventLoopGuard(event_loop_lock))
}


pub async fn device_poller(id: i32, mut device_info_in: tokio::sync::watch::Receiver<Option<DeviceInfo>>, shutdown: Arc<tokio::sync::Notify>, close_on_dc: bool) {
    loop  {
        let dev_info = match device_info_in.changed().await {
            Ok(_) => {
                match device_info_in.borrow_and_update().clone() {
                    Some(d) => d,
                    None => { continue; }
                }
            }
            Err(_e) => { break; }
        };
        //let Some(dev_info) = device_info_in.recv().await


        let device_id = dev_info.id();
        let Ok((mut host, channels)) = RdxUsbFsHost::open_device(dev_info, 32).await else { continue; };
        let (mut write_poller, writer) = host.write_poller(32);


        let open_device = OpenDevice {
            channels: DeviceChannels::FsDevice(channels),
            writer: Writer::FsDevice(writer),
            device_id,
            protocol: 0,
        };
        {
            let mut event_loop = acquire_event_loop();
            event_loop.update_open_device(id, open_device);
        }
        // this will eventually error out on disconnect
        tokio::select! {
            val = host.poll(32, true) => { val.ok(); }
            val = write_poller.poll() => { val.ok(); }
            // we need a semaphore here because oneshot channels won't live on repeat iterations
            _val = shutdown.notified() => { return; }
        }
        {
            let mut event_loop = acquire_event_loop();
            event_loop.remove_open_device(id);
            if close_on_dc {
                // TODO: close bus
                event_loop.devices.remove(&id);
                return;
            }
        }
    }
}


pub async fn hotplug() {
    let mut hotplug_watcher = nusb::watch_devices().unwrap();
    while let Some(event) = hotplug_watcher.next().await {
        match event {
            nusb::hotplug::HotplugEvent::Connected(device_info) => {
                let mut event_loop = acquire_event_loop();
                'device_iter: for device in event_loop.devices.values_mut() {
                    if device.matches_device_info(&device_info) {
                        device.device_info_out.send_replace(Some(device_info));
                        break 'device_iter;
                    }
                }
            }
            nusb::hotplug::HotplugEvent::Disconnected(_device_id) => {}
        }
    }
}

pub fn force_scan_devices(event_loop: EventLoopGuard) -> Result<EventLoopGuard, EventLoopError> {
    let Ok(list_device_iter) = nusb::list_devices() else { return Err(EventLoopError::CannotListDevices); };
    for device_info in list_device_iter {
        'device_loop: for device in event_loop.devices.values() {
            if device.matches_device_info(&device_info) {
                device.device_info_out.send_replace(Some(device_info));
                break 'device_loop;
            }
        }
    }
    Ok(event_loop)
}

pub fn open_device(vid: u16, pid: u16, serial_number: Option<String>, close_on_dc: bool) -> Result<i32, EventLoopError> {
    let mut event_loop = try_acquire_event_loop()?;

    let maybe_existing = event_loop.devices.iter_mut().find_map(|(handle, device)| {
        if device.matches(vid, pid, serial_number.as_ref().map(|s| s.as_str())) {
            Some(*handle)
        } else { None }
    });
    if let Some(existing_handle) = maybe_existing {
        force_scan_devices(event_loop)?;
        return Ok(existing_handle);
    }

    let (tx, rx) = tokio::sync::watch::channel(None);

    // nothing matches, let's add a device
    let handle = event_loop.next_handle;
    event_loop.next_handle += 1;
    let shutdown = Arc::new(tokio::sync::Notify::new());

    let device_poller_task = event_loop.rt.spawn(device_poller(handle, rx, shutdown.clone(), close_on_dc));
    let device_entry = Device {
        vid,
        pid,
        serial_number,
        handle: None,
        device_info_out: tx,
        poller_handle: device_poller_task,
        shutdown,
    };

    event_loop.devices.insert(handle, device_entry);
    force_scan_devices(event_loop)?;
    Ok(handle)
}

pub fn read_packets(handle_id: i32, channel: u8, packets: &mut [RdxUsbPacket]) -> Result<usize, EventLoopError> {
    let mut event_loop = try_acquire_event_loop()?;
    let open_device = event_loop.acquire_open_device(handle_id)?;

    let mut packets_read = 0usize;

    for packet in packets {
        *packet = match open_device.try_read(channel) {
            Ok(p) => {
                packets_read += 1;
                p.into()
            }
            Err(e) => match e {
                DeviceIOError::ChannelOutOfRange => { return Err(EventLoopError::ChannelOutOfRange); }
                DeviceIOError::NoData => { break; }
            }
        }
    }
    Ok(packets_read)
}

pub fn write_packets(handle_id: i32, packets: &[RdxUsbPacket]) -> Result<usize, EventLoopError> {
    let mut event_loop = try_acquire_event_loop()?;
    let open_device = event_loop.acquire_open_device(handle_id)?;
    let mut packets_written = 0usize;

    for packet in packets {
        match open_device.try_write(packet) {
            Ok(_) => {
                packets_written += 1;
            }
            Err(_) => { break; }
        }
    }

    Ok(packets_written)
}

pub fn close_device(handle_id: i32) -> Result<(), EventLoopError> {
    let mut event_loop = try_acquire_event_loop()?;
    let Some(device) = event_loop.devices.get_mut(&handle_id) else { return Ok(()); };
    device.shutdown.notify_one();
    event_loop.devices.remove(&handle_id);
    Ok(())
}

pub fn close_all_devices() -> Result<(), EventLoopError> {
    let mut event_loop = try_acquire_event_loop()?;
    event_loop.devices.retain(|_handle, device| {
        device.shutdown.notify_one();
        false
    });
    Ok(())
}