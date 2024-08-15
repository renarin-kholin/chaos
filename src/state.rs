use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

pub type UserId = String;
pub type SDP = String;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ChaosMessage {
    pub client_id: String,
    pub message_content: String,
}
#[derive(Default, Serialize, Deserialize, Debug, PartialEq, Eq, Copy, Clone)]
pub enum ConnectionProgress {
    #[default]
    Closed,
    CallRequestSent,
    CallRequestReceived,
    CallRequestAccepted,
    CallAnswerSent,
    CallAnswerReceived,
    CallReplySent,
    CallReplyReceived,
    Established,
}

#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct ConnectionDetails {
    pub id: UserId,
    pub sdp: SDP,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Connection {
    remote: ConnectionDetails,
    messages: Vec<ChaosMessage>,
    pub progress: ConnectionProgress,
}

impl Connection {
    pub fn new(remote_id: UserId) -> Self {
        Self {
            remote: ConnectionDetails {
                id: remote_id,
                sdp: Default::default(),
            },
            messages: Default::default(),
            progress: Default::default(),
        }
    }
    pub fn set_progress(&mut self, progress: ConnectionProgress) {
        self.progress = progress;
    }
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IndependentState {
    pub connection_details: ConnectionDetails,
    pub connections: HashMap<UserId, Connection>,
}
impl Default for IndependentState {
    fn default() -> Self {
        Self {
            connection_details: ConnectionDetails::default(),
            connections: Default::default(),
        }
    }
}
pub type SharedState = Arc<RwLock<IndependentState>>;
//This contains the placeholders for gui inputs
#[derive(Default, Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum SidebarButton {
    #[default]
    NewConnection,
    Chat(UserId),
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct GUIState {
    pub remote_id: String,
    pub current_message: String,
    pub current_sidebar_button: SidebarButton,
    pub display_state: IndependentState,
}
