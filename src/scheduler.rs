use std::{collections::HashMap, sync::Arc};

use crossbeam_channel::{Receiver, Sender};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};

use crate::peer;
use crate::state::{Connection, ConnectionProgress, UserId, SDP};
use crate::{state::IndependentState, utils::Attach};

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub enum GUICommand {
    CallRequest(UserId),
    CallAnswer(bool, UserId),
    UpdateState(IndependentState),
}
#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub enum WSCommand {
    SetClientId(UserId),
    CallRequest(UserId),
    CallRequestFailure,
    CallAnswer(bool, Option<SDP>),
    CallReply(SDP),
}
#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub enum PeerCommand {
    NewPeerConnection(UserId),
    CallAnswer(SDP),

    EstablishConnection(SDP, bool),
    CallReply(SDP),
}
#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub enum StateCommand {
    SetClientId(UserId),
    SetProgress(UserId, ConnectionProgress),
}

#[derive(PartialEq, Debug, Deserialize, Serialize)]
pub enum Command {
    GUI(GUICommand),
    WS(WSCommand),
    State(StateCommand),
    Peer(PeerCommand),
}

#[derive(PartialEq, Eq, Hash)]
pub enum ThreadTypes {
    Coupler,
    Datachannel,
    GUI,
    Scheduler,
    Peer,
}
pub type ChannelAttachment = (Sender<Command>, Receiver<Command>);

pub struct Scheduler {
    attachments: Arc<Mutex<HashMap<ThreadTypes, ChannelAttachment>>>,
    independent_state: Arc<RwLock<IndependentState>>,
}
impl Scheduler {
    pub fn new(state: Arc<RwLock<IndependentState>>) -> Self {
        Self {
            attachments: Arc::new(Mutex::new(HashMap::new())),

            independent_state: state,
        }
    }
    pub fn run(&mut self) {
        let attachments = &self.attachments.clone();
        let independent_state = &self.independent_state.clone();
        for (thread_type, attachment) in attachments.try_lock().unwrap().iter() {
            let attachments = attachments.clone();
            let attachment = attachment.clone();
            let independent_state = independent_state.clone();
            tokio::spawn(async move {
                let (_, rx) = attachment.clone();

                while let Ok(command) = rx.recv() {
                    println!("Received a command, {:?}", command);
                    match command {
                        Command::GUI(gui_command) => match gui_command {
                            GUICommand::CallRequest(remote_id) => {
                                let attachments = attachments.try_lock().unwrap();
                                let (tx, _) = attachments.get(&ThreadTypes::Coupler).unwrap();
                                tx.try_send(Command::WS(WSCommand::CallRequest(remote_id)))
                                    .unwrap();
                            }
                            GUICommand::CallAnswer(accepted, remote_id) => {
                                let attachments = attachments.try_lock().unwrap();

                                let state = independent_state.clone();

                                if accepted {
                                    {
                                        let mut state = state.try_write().unwrap();
                                        let connection = state.connections.get_mut(&remote_id);
                                        if let Some(mut connection) = connection {
                                            connection
                                                .set_progress(ConnectionProgress::CallAnswerSent);
                                        }
                                        let (tx, _) = attachments.get(&ThreadTypes::GUI).unwrap();
                                        tx.try_send(Command::GUI(GUICommand::UpdateState(
                                            state.clone(),
                                        )))
                                        .unwrap();
                                    }
                                    let (tx_peer, _) = attachments.get(&ThreadTypes::Peer).unwrap();
                                    tx_peer
                                        .try_send(Command::Peer(PeerCommand::NewPeerConnection(
                                            remote_id.clone(),
                                        )))
                                        .unwrap();
                                } else {
                                    {
                                        let mut state = state.try_write().unwrap();
                                        let connection = state.connections.get_mut(&remote_id);
                                        if let Some(mut connection) = connection {
                                            connection.set_progress(ConnectionProgress::Closed);
                                        }
                                        let (tx, _) = attachments.get(&ThreadTypes::GUI).unwrap();
                                        tx.try_send(Command::GUI(GUICommand::UpdateState(
                                            state.clone(),
                                        )))
                                        .unwrap();
                                    }
                                    let (tx_coupler, _) =
                                        attachments.get(&ThreadTypes::Coupler).unwrap();
                                    tx_coupler
                                        .try_send(Command::WS(WSCommand::CallAnswer(false, None)))
                                        .unwrap();
                                }
                            }
                            _ => {}
                        },
                        Command::State(state_command) => match state_command {
                            StateCommand::SetClientId(client_id) => {
                                let state = independent_state.clone();
                                let mut state = state.try_write().unwrap();
                                state.connection_details.id = client_id;
                                let attachments = attachments.try_lock().unwrap();
                                let (tx, _) = attachments.get(&ThreadTypes::GUI).unwrap();
                                tx.try_send(Command::GUI(GUICommand::UpdateState(state.clone())))
                                    .unwrap();
                            }
                            StateCommand::SetProgress(remote_id, progress) => {
                                println!("Connection Progress Updated: {:?}", progress);
                                let state = independent_state.clone();
                                let mut state = state.try_write().unwrap();
                                let connection = state.connections.get_mut(&remote_id);
                                if let Some(mut connection) = connection {
                                    connection.set_progress(progress);
                                } else {
                                    let mut connection = Connection::new(remote_id.clone());
                                    connection.set_progress(progress);
                                    state.connections.insert(remote_id, connection);
                                }
                                let attachments = attachments.try_lock().unwrap();
                                let (tx, _) = attachments.get(&ThreadTypes::GUI).unwrap();
                                tx.try_send(Command::GUI(GUICommand::UpdateState(state.clone())))
                                    .unwrap();
                            }
                            _ => {
                                println!("Not implemented yet");
                            }
                        },
                        Command::WS(ws_command) => match ws_command {
                            WSCommand::CallAnswer(accepted, remote_sdp) => {
                                let attachments = attachments.try_lock().unwrap();
                                if (accepted) {
                                    let (tx_peer, _) = attachments.get(&ThreadTypes::Peer).unwrap();
                                    tx_peer
                                        .try_send(Command::Peer(PeerCommand::EstablishConnection(
                                            remote_sdp.unwrap(),
                                            false,
                                        )))
                                        .unwrap();
                                }
                            }
                            WSCommand::CallReply(remote_sdp) => {
                                let attachments = attachments.try_lock().unwrap();
                                let (tx_peer, _) = attachments.get(&ThreadTypes::Peer).unwrap();
                                tx_peer
                                    .try_send(Command::Peer(PeerCommand::EstablishConnection(
                                        remote_sdp, true,
                                    )))
                                    .unwrap();
                            }
                            _ => {
                                println!("Not implemented yet.");
                            }
                        },
                        Command::Peer(peer_command) => match peer_command {
                            PeerCommand::CallAnswer(local_sdp) => {
                                let attachments = attachments.try_lock().unwrap();
                                let (tx, _) = attachments.get(&ThreadTypes::Coupler).unwrap();
                                tx.try_send(Command::WS(WSCommand::CallAnswer(
                                    true,
                                    Some(local_sdp),
                                )))
                                .unwrap();
                            }
                            PeerCommand::CallReply(local_sdp) => {
                                let attachments = attachments.try_lock().unwrap();
                                let (tx, _) = attachments.get(&ThreadTypes::Coupler).unwrap();
                                tx.try_send(Command::WS(WSCommand::CallReply(local_sdp)))
                                    .unwrap();
                            }
                            _ => {
                                println!("Not implemented yet.");
                            }
                        },
                        _ => {
                            println!("Not implemented yet");
                        }
                    }
                }
            });
        }
    }
}
impl Attach for Scheduler {
    fn attach(&mut self, attachment: ChannelAttachment, thread: Option<ThreadTypes>) {
        self.attachments.try_lock().unwrap().insert(
            thread.expect("Scheduler attachment provided with no thread type"),
            attachment,
        );
    }
}
