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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Pod, Zeroable)]
#[repr(C, packed)]
pub struct RdxUsbPacket {
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
    /// data (max size: 64 bytes)
    pub data: [u8; 64]
}

impl From<RdxUsbFsPacket> for RdxUsbPacket {
    fn from(value: RdxUsbFsPacket) -> Self {
        let mut data = [0u8; 64];
        data[..48].copy_from_slice(&value.data);
        Self {
            timestamp_ns: value.timestamp_ns,
            arb_id: value.arb_id,
            dlc: value.dlc,
            channel: value.channel,
            flags: value.flags,
            data,
        }
    }
}

impl TryFrom<RdxUsbPacket> for RdxUsbFsPacket {
    type Error = RdxUsbPacket;

    fn try_from(value: RdxUsbPacket) -> Result<Self, Self::Error> {
        if value.dlc > 48 { return Err(value); }
        let mut data = [0u8; 48];
        data.copy_from_slice(&value.data[..value.dlc as usize]);
        Ok(RdxUsbFsPacket {
            timestamp_ns: value.timestamp_ns,
            arb_id: value.arb_id,
            dlc: value.dlc,
            channel: value.channel,
            flags: value.flags,
            data,
        })
        
    }
}

impl RdxUsbFsPacket {
    pub const fn encode(self) -> [u8; 64] {
        unsafe { core::mem::transmute(self) }
    }

    pub const fn from_buf(data: [u8; 64]) -> Self {
        unsafe { core::mem::transmute(data) }
    }

    pub const fn id(&self) -> u32 {
        self.arb_id & 0x1fff_ffff
    }

    pub const fn extended(&self) -> bool {
        self.arb_id & MESSAGE_FLAG_EFF != 0
    }

    pub const fn rtr(&self) -> bool {
        self.arb_id & MESSAGE_FLAG_RTR != 0
    }

    pub const fn err(&self) -> bool {
        self.arb_id & MESSAGE_FLAG_ERR != 0
    }

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