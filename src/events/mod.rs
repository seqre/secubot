#[cfg(feature = "msg_content")]
mod message;

#[cfg(feature = "msg_content")]
pub use message::handle_message;
