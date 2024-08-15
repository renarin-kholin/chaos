use crate::scheduler::{ChannelAttachment, ThreadTypes};

pub mod crypto;
pub mod message;
pub mod webrtc;

pub trait Attach<T = ChannelAttachment> {
    fn attach(&mut self, channel_attachment: T, thread: Option<ThreadTypes>);
}
pub trait ChaosThread {
    fn start();
}
