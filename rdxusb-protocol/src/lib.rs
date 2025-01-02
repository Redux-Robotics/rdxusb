#![no_std]

use bytemuck::{Pod, Zeroable};

/// In bulk xfer endpoint (has top bit set)
pub const ENDPOINT_IN: u8 = 0x81;
/// Out bulk xfer endpoint
pub const ENDPOINT_OUT: u8 = 0x02;

/// this bit is true on arbitration IDs [`RdxUsbFsPacket::arb_id`] that are extended (29-bit).
pub const MESSAGE_ARB_ID_EXT: u32 = 0x80000000;
/// this bit is true on arbitration IDs [`RdxUsbFsPacket::arb_id`] associated with an RTR frame.
pub const MESSAGE_ARB_ID_RTR: u32 = 0x40000000;
/// Specifies the frame is specifically addressed to/from the device.
///
/// For messages from device to host, this means that the message in fact originates from the device, 
/// and not any connected devices proxied through other buses.
///
/// For messages from host to device, the device will understand that the host message is meant for it,
/// regardless of any configured device id bits.
pub const MESSAGE_ARB_ID_DEVICE: u32 = 0x20000000;


/// Data packet passed to USB-full-speed devices which have a max packet size of 64.
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

/// Generic data packet passed to/from RdxUsb APIs.
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
        let len = value.dlc as usize;
        let mut data = [0u8; 48];
        data[..len].copy_from_slice(&value.data[..len]);
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
    /// The message arbitration id
    pub const fn id(&self) -> u32 {
        self.arb_id & 0x1fff_ffff
    }

    /// Does the packet use extended (29-bit) IDs?
    pub const fn extended(&self) -> bool {
        self.arb_id & MESSAGE_ARB_ID_EXT != 0
    }

    /// Is the packet an RTR packet?
    pub const fn rtr(&self) -> bool {
        self.arb_id & MESSAGE_ARB_ID_RTR != 0
    }

    /// Is the packet a device packet?
    pub const fn device(&self) -> bool {
        self.arb_id & MESSAGE_ARB_ID_DEVICE != 0
    }

    /// Should always be 64.
    pub const SIZE: usize = core::mem::size_of::<Self>();
}

/// Struct returned by the device info control request
#[derive(Debug, PartialEq, Eq, Clone, Copy, Pod, Zeroable)]
#[repr(C, packed)]
pub struct RdxUsbDeviceInfo {
    /// The SKU index of the device (the first number in the serial)
    pub sku: u16,
    /// The interface index that the RdxUSB interface uses
    pub interface_idx: u8,
    /// The number of channels that the RdxUSB interface supports (0-indexed)
    pub n_channels: u8,
    /// The major protocol version
    pub protocol_version_major: u16,
    /// The minor protocol version
    pub protocol_version_minor: u16,
    /// Reserved bits
    pub reserved: [u8; 24]
}

/// Control requests supported
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum RdxUsbCtrl {
    DeviceInfo = 0,
}

/// USB-Full Speed protocol version
pub const PROTOCOL_VERSION_FS: u16 = 1;