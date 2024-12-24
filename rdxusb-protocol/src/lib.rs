#![no_std]

use bytemuck::{Pod, Zeroable};

/// In bulk xfer endpoint (has top bit set)
pub const ENDPOINT_IN: u8 = 0x81;
/// Out bulk xfer endpoint
pub const ENDPOINT_OUT: u8 = 0x02;

/// this bit is true on arbitration IDs [`RdxUsbFsPacket::arb_id`] that are extended (29-bit).
pub const MESSAGE_FLAG_EFF: u32 = 0x80000000;
/// this bit is true on arbitration IDs [`RdxUsbFsPacket::arb_id`] associated with an RTR frame.
pub const MESSAGE_FLAG_RTR: u32 = 0x40000000;
/// this bit is true on arbitration IDs [`RdxUsbFsPacket::arb_id`] associated with an error frame.
pub const MESSAGE_FLAG_ERR: u32 = 0x20000000;


#[derive(Debug, PartialEq, Eq, Clone, Copy, Pod, Zeroable)]
#[repr(C, packed)]
pub struct RdxUsbFsPacket {
    /// Timestamp since boot (nanoseconds)
    pub timestamp_ns: u64,
    /// CAN arbitration id.
    pub arb_id: u32, // CAN arbitration id. 
    /// Data length code.
    pub dlc: u8,
    /// Relevant channel. Zero most of the time.
    pub channel: u8,
    /// Misc flags (unused for now)
    pub flags: u16,
    /// data (max size: 48 bytes)
    pub data: [u8; 48]
}

impl RdxUsbFsPacket {
    pub const SIZE: usize = core::mem::size_of::<Self>();
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Pod, Zeroable)]
#[repr(C, packed)]
pub struct RdxUsbDeviceInfo {
    pub sku: u16,
    pub protocol: u8,
    pub n_channels: u8,
    pub reserved: [u8; 28]
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum RdxUsbCtrl {
    DeviceInfo = 0,
    ResetChannel = 1,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum RdxUsbProtocol {
    /// Supports the baseline bulk xfer USB-Full Speed protocol (version 1)
    FsProtocol = 0,
}