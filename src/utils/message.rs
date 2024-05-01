use serde::{Deserialize, Serialize};

type UserId = String;

type SDP = String;

#[derive(Serialize, Deserialize, Debug)]
pub enum CCMessage {
    SetClientId(UserId),
    CallRequest(UserId),
    CallAnswer(bool, Option<SDP>),
    CallReply(SDP),
    CallRequestFailure,
}

pub enum CCErrors {
    CallRequestFailure,
    None,
}
impl CCErrors {
    pub fn default() -> Self {
        Self::None
    }
}
