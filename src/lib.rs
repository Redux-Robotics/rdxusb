pub mod host;
/// Integrated tokio-driven event loop that handles hotplug and polling logic automatically.
/// This is the backend used for the C API.
#[cfg(feature = "event-loop")]
pub mod event_loop;
/// An abstracted C API used for everything else.
#[cfg(feature = "c-api")]
pub mod c_api;

pub use rdxusb_protocol::{RdxUsbPacket, MESSAGE_ARB_ID_DEVICE, MESSAGE_ARB_ID_EXT, MESSAGE_ARB_ID_RTR};