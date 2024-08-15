use crate::state::ChaosMessage;

use super::message::CCMessage;

pub enum CommunicationType {
    MakeConnection(String),
    SendMessage(ChaosMessage),
    SetClientSDP(String),
    SendCCMessage(CCMessage),
}
