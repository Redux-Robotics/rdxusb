pub mod host;
/// Integrated event loop for C consumers
#[cfg(feature = "event-loop")]
pub(crate) mod event_loop;
#[cfg(feature = "c-api")]
pub mod c_api;