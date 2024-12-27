#![allow(dead_code)]


//use std::time::Instant;


use std::fmt::Display;

use bytemuck::AnyBitPattern;
use futures_util::StreamExt;
use nusb::{transfer::{ControlIn, ControlOut, ControlType, Recipient, RequestBuffer}, DeviceInfo};
use rdxusb_protocol::{RdxUsbCtrl, RdxUsbDeviceInfo, RdxUsbFsPacket, ENDPOINT_OUT};
use ringbuf::{storage::Heap, traits::Consumer};
use async_ringbuf::{traits::{AsyncProducer, AsyncConsumer, Producer, Split}, AsyncHeapRb, AsyncRb};

/*

for channel in channels:


pool:
 - acquire all free vecs

client:
 - in-queue of read vecs
 - await on queue, obtain vec
 - move vec back to pool


*/

/// USB full-speed spec host.
pub struct RdxUsbFsHost {
    iface: nusb::Interface,
    n_channels: u8,
    // we need this secondary queue because gs_usb only has one rx queue for all channels
    // so we need to split it up.
    // it is the responsibility of the owner of GsUsbDevice to await on poll() until complete
    //rx_queue: Vec<tokio::sync::mpsc::Sender<Vec<u8>>>,
    rx_queue: Vec<<AsyncRb<Heap<RdxUsbFsPacket>> as async_ringbuf::traits::Split>::Prod>
}

#[derive(Debug)]
pub enum RdxUsbHostError {
    UnsupportedProtocol,
    InvalidChannel,
    NoInterface,
    NusbError(nusb::Error),
    TransferCancelled, 
    EndpointStall,
    DeviceDisconnected,
    UsbFault,
    TransferUnknownError,
    DataDecodeError,
}

impl From<nusb::Error> for RdxUsbHostError {
    fn from(value: nusb::Error) -> Self {
        Self::NusbError(value)
    }
}

impl From<nusb::transfer::TransferError> for RdxUsbHostError {
    fn from(value: nusb::transfer::TransferError) -> Self {
        match value {
            nusb::transfer::TransferError::Cancelled => RdxUsbHostError::TransferCancelled,
            nusb::transfer::TransferError::Stall => RdxUsbHostError::EndpointStall,
            nusb::transfer::TransferError::Disconnected => RdxUsbHostError::DeviceDisconnected,
            nusb::transfer::TransferError::Fault => RdxUsbHostError::UsbFault,
            nusb::transfer::TransferError::Unknown => RdxUsbHostError::TransferUnknownError,
        }
    }
}

impl From<bytemuck::PodCastError> for RdxUsbHostError {
    fn from(_value: bytemuck::PodCastError) -> Self {
        RdxUsbHostError::DataDecodeError
    }
}

impl Display for RdxUsbHostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RdxUsbHostError::UnsupportedProtocol => write!(f, "Unsupported protocol"),
            RdxUsbHostError::InvalidChannel => write!(f, "Invalid channel"),
            RdxUsbHostError::NoInterface => write!(f, "No valid USB interface"),
            RdxUsbHostError::NusbError(error) => write!(f, "nusb error: {error}"),
            RdxUsbHostError::TransferCancelled => write!(f, "Transfer cancelled"),
            RdxUsbHostError::EndpointStall => write!(f, "Endpoint stall"),
            RdxUsbHostError::DeviceDisconnected => write!(f, "Device disconnected"),
            RdxUsbHostError::UsbFault => write!(f, "USB fault"),
            RdxUsbHostError::TransferUnknownError => write!(f, "Unknown transfer error"),
            RdxUsbHostError::DataDecodeError => write!(f, "Received undecodable data"),
        }
    }
}
impl core::error::Error for RdxUsbHostError {}


pub type RdxUsbHostResult<T> = Result<T, RdxUsbHostError>;

impl RdxUsbFsHost {
    /// Opens the device with the [`DeviceInfo`] and specified rx queue buffer size.
    /// Returns a usb device handle
    pub async fn open_device(dev_info: DeviceInfo, rx_q_size: usize) -> RdxUsbHostResult<(Self, Vec<RdxUsbFsChannel>)> {

        let Some(iface) = dev_info.interfaces().find(|iface| {
            iface.class() == 0xff && iface.subclass() == 0x0 && iface.protocol() == 0x0
        }) else { return Err(RdxUsbHostError::NoInterface); };

        let iface_idx = iface.interface_number();

        let handle = dev_info.open()?;
        handle.detach_kernel_driver(iface_idx)?;
        let iface = handle.claim_interface(iface_idx)?;
        let cfg = Self::get_device_info(&iface).await?;
        let icount = cfg.n_channels;

        // TODO: split into RdxUsbFsHost or RdxUsbHsHost here.

        let mut dev = RdxUsbFsHost {
            iface: iface.clone(),
            n_channels: icount,
            rx_queue: Vec::with_capacity(icount as usize),
        };

        let mut v = Vec::with_capacity(icount as usize);
        for i in 0..=icount {
            //let (tx, rx) = tokio::sync::mpsc::channel(rx_q_size);
            let (prod, cons) = AsyncHeapRb::new(rx_q_size).split();

            v.push(RdxUsbFsChannel {
                iface: iface.clone(),
                channel: i,
                rx_queue: cons,
            });
            dev.rx_queue.push(prod);
        }

        Ok((dev, v))
    }

    /// This drives the event loop.
    /// 
    /// **n_transfers** determines the maximum number of transfers to be flighted at a time.
    pub async fn poll(&mut self, n_transfers: usize, await_on_full: bool) -> RdxUsbHostResult<()> {
        let mut read_queue = self.iface.bulk_in_queue(rdxusb_protocol::ENDPOINT_IN);

        while read_queue.pending() < n_transfers {
            read_queue.submit(RequestBuffer::new(RdxUsbFsPacket::SIZE))
        }

        loop {
            let buf = read_queue.next_complete().await.into_result()?;
            //println!("Received message: len={} {buf:?}", buf.len());
            if let Ok(pkt) = bytemuck::try_from_bytes::<RdxUsbFsPacket>(buf.as_slice()) {
                if (pkt.channel as usize) < self.rx_queue.len() {
                    if await_on_full {
                        self.rx_queue[pkt.channel as usize].push(pkt.clone()).await.ok();
                    } else {
                        self.rx_queue[pkt.channel as usize].try_push(pkt.clone()).ok();
                    }
                }
            } 

            read_queue.submit(RequestBuffer::reuse(buf, RdxUsbFsPacket::SIZE))
        }
        //println!("Packet id: {:#08x} ts: {}", header.arbitration_id(), u32::from_le_bytes(buf[20..24].try_into().unwrap()));
    }

    async fn get_device_info(iface: &nusb::Interface) -> RdxUsbHostResult<RdxUsbDeviceInfo> {
        let res = iface.control_in(ControlIn { 
            control_type: ControlType::Vendor,
            recipient: Recipient::Interface,
            request: RdxUsbCtrl::DeviceInfo as u8,
            value: 1,
            index: 0,
            length: core::mem::size_of::<RdxUsbDeviceInfo>() as u16,
        }).await.into_result()?;
        Ok(bytemuck::try_from_bytes::<RdxUsbDeviceInfo>(&res.as_slice())?.clone())
    }

    pub async fn get_device_config(&self) -> RdxUsbHostResult<RdxUsbDeviceInfo> {
        Self::get_device_info(&self.iface).await
    }

    pub fn write_poller(&self, n_packets: usize) -> (RdxUsbFsWritePoller, RdxUsbFsWriter) {
        RdxUsbFsWritePoller::new(self.iface.clone(), n_packets)
    }

}

pub struct RdxUsbFsWriter(<AsyncRb<Heap<RdxUsbFsPacket>> as async_ringbuf::traits::Split>::Prod);

impl RdxUsbFsWriter {
    pub fn try_send(&mut self, packet: RdxUsbFsPacket) -> Option<RdxUsbFsPacket> {
        self.0.try_push(packet).err()
    }
    pub async fn send(&mut self, packet: RdxUsbFsPacket) -> Result<(), RdxUsbFsPacket> {
        self.0.push(packet).await
    }
}

pub struct RdxUsbFsWritePoller {
    iface: nusb::Interface,
    tx_queue: <AsyncRb<Heap<RdxUsbFsPacket>> as async_ringbuf::traits::Split>::Cons,
}

impl RdxUsbFsWritePoller {
    pub fn new(iface: nusb::Interface, n_packets: usize) -> (Self, RdxUsbFsWriter) {
        let (prod, cons) = AsyncHeapRb::new(n_packets).split();

        (Self { iface, tx_queue: cons, }, RdxUsbFsWriter(prod))
    }

    pub async fn poll(&mut self) -> Result<(), RdxUsbHostError> {
        let mut buffer= Vec::with_capacity(64);
        while let Some(msg) = self.tx_queue.next().await {
            buffer.clear();
            buffer.extend_from_slice(&msg.encode());
            buffer = self.iface.bulk_out(ENDPOINT_OUT, buffer).await.into_result()?.reuse();
        }
        Ok(())
    }
}


pub struct RdxUsbFsChannel {
    iface: nusb::Interface,
    channel: u8,
    rx_queue: <AsyncRb<Heap<RdxUsbFsPacket>> as async_ringbuf::traits::Split>::Cons,
}

impl RdxUsbFsChannel {
    pub async fn control_in_struct<T: AnyBitPattern>(&self, req: RdxUsbCtrl) -> RdxUsbHostResult<T> {
        let res = self.iface.control_in(ControlIn {
            control_type: ControlType::Vendor,
            recipient: Recipient::Interface,
            request: req as u8,
            value: self.channel as u16,
            index: 0,
            length: core::mem::size_of::<T>() as u16,
        }).await.into_result()?;
        Ok(bytemuck::try_from_bytes::<T>(&res.as_slice())?.clone())
    }

    pub async fn control_out_struct(&self, req: RdxUsbCtrl, data: &[u8]) -> RdxUsbHostResult<()> {
        self.iface.control_out(ControlOut {
            control_type: ControlType::Vendor,
            recipient: Recipient::Interface,
            request: req as u8,
            value: self.channel as u16,
            index: 0,
            data,
        }).await.into_result()?;
        Ok(())
    }

    pub fn interface(&self) -> &nusb::Interface {
        &self.iface
    }

    pub async fn read(&mut self) -> RdxUsbHostResult<RdxUsbFsPacket> {
        match self.rx_queue.pop().await {
            Some(v) => Ok(v),
            None => Err(RdxUsbHostError::DeviceDisconnected)
        }
    }

    pub fn try_read(&mut self) -> Option<RdxUsbFsPacket> {
        self.rx_queue.try_pop()
    }

    pub async fn write(&mut self, mut pkt: RdxUsbFsPacket) -> RdxUsbHostResult<()> {
        pkt.channel = self.channel;
        let v = Vec::from(bytemuck::bytes_of(&pkt));
        self.iface.bulk_out(rdxusb_protocol::ENDPOINT_OUT, v).await.into_result()?;
        Ok(())
    }

    pub async fn write_buf(&mut self, vbuf: Vec<u8>) -> RdxUsbHostResult<Vec<u8>> {
        Ok(self.iface.bulk_out(rdxusb_protocol::ENDPOINT_OUT, vbuf).await.into_result()?.reuse())
    }
}
