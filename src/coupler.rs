use std::sync::Arc;

use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::Message::Text;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use url::Url;

use crate::scheduler::{Command, StateCommand, WSCommand};
use crate::state::ConnectionProgress;
use crate::state::ConnectionProgress::*;
use crate::{
    scheduler::{ChannelAttachment, ThreadTypes},
    utils::Attach,
};

pub type SplitSocketRead = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;
pub type SplitSocketWrite = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
pub struct Coupler {
    attachment: Arc<Mutex<ChannelAttachment>>,
    socket: (Arc<Mutex<SplitSocketWrite>>, Arc<Mutex<SplitSocketRead>>),
}

impl Coupler {
    pub async fn new(attachment: Arc<Mutex<ChannelAttachment>>) -> Self {
        let (ws_socket, _response) =
            connect_async(Url::parse("ws://localhost:3030/couple").unwrap())
                .await
                .unwrap();
        let (ws_write, ws_read) = ws_socket.split();
        let socket = (
            Arc::new(Mutex::new(ws_write)),
            Arc::new(Mutex::new(ws_read)),
        );
        Self { attachment, socket }
    }
    pub async fn start(&mut self) {
        let socket = self.socket.clone();
        let (socket_write, socket_read) = socket.clone();
        let attachment = self.attachment.clone();
        let attachment2 = Arc::clone(&attachment);
        tokio::spawn(async move {
            websocket_thread(attachment, socket_read).await;
            println!("Websocket Listener thread closed.");
        });

        tokio::spawn(async move {
            coupler_scheduler_thread(attachment2, socket_write).await;
            println!("Coupler-Scheduler Thread closed.");
        });
    }
}
impl Attach<Arc<Mutex<ChannelAttachment>>> for Coupler {
    fn attach(
        &mut self,
        channel_attachment: Arc<Mutex<ChannelAttachment>>,
        _: Option<ThreadTypes>,
    ) {
        self.attachment = channel_attachment;
    }
}

async fn websocket_thread(
    attachment: Arc<Mutex<ChannelAttachment>>,
    socket_read: Arc<Mutex<SplitSocketRead>>,
) {
    let attachment = attachment.clone();
    while let Ok(ws_message) = socket_read.try_lock().unwrap().next().await.unwrap() {
        let ws_command = serde_json::from_str::<WSCommand>(ws_message.to_text().unwrap())
            .expect("Invalid websocket message");
        println!("Websocket Message: {}", ws_message);
        let attachment = attachment.clone();
        use WSCommand::*;
        match ws_command {
            SetClientId(client_id) => {
                let (tx, _) = attachment.try_lock().unwrap().clone();
                tx.try_send(Command::State(StateCommand::SetClientId(client_id)))
                    .unwrap();
            }
            CallRequest(remote_id) => {
                //ask user permission

                let (tx, _) = attachment.try_lock().unwrap().clone();
                tx.try_send(Command::State(StateCommand::SetProgress(
                    remote_id.clone(),
                    ConnectionProgress::CallRequestReceived,
                )))
                .unwrap();
            }
            CallAnswer(accepted, Some(remote_sdp)) => {
                let (tx, _) = attachment.try_lock().unwrap().clone();
                tx.try_send(Command::WS(WSCommand::CallAnswer(
                    accepted,
                    Some(remote_sdp),
                )))
                .unwrap();
            }
            CallReply(remote_sdp) => {
                let (tx, _) = attachment.try_lock().unwrap().clone();
                tx.try_send(Command::WS(WSCommand::CallReply(remote_sdp)))
                    .unwrap();
            }
            _ => {
                println!("Not implemented yet.");
            }
        };
    }
}
async fn coupler_scheduler_thread(
    attachment: Arc<Mutex<ChannelAttachment>>,
    socket_write: Arc<Mutex<SplitSocketWrite>>,
) {
    let (_, rx) = attachment.try_lock().unwrap().clone();
    while let Ok(command) = rx.recv() {
        if let Command::WS(ws_command) = command {
            use WSCommand::*;
            match ws_command {
                CallRequest(remote_id) => {
                    let msg = WSCommand::CallRequest(remote_id.clone());
                    let msg_str = serde_json::to_string(&msg).unwrap();
                    let (tx, _) = attachment.try_lock().unwrap().clone();
                    tx.try_send(Command::State(StateCommand::SetProgress(
                        remote_id.clone(),
                        CallRequestSent,
                    )))
                    .unwrap();
                    websocket_send(socket_write.clone(), msg_str).await;
                }
                CallAnswer(accepted, sdp) => {
                    let msg = WSCommand::CallAnswer(accepted, sdp);
                    let msg_str = serde_json::to_string(&msg).unwrap();
                    let (tx, _) = attachment.try_lock().unwrap().clone();
                    // tx.try_send(Command::State(StateCommand::SetProgress(
                    //     remote_id.clone(),
                    //     CallAnswerSent,
                    // )))
                    // .unwrap();
                    websocket_send(socket_write.clone(), msg_str).await;
                }
                CallReply(local_sdp) => {
                    let msg = WSCommand::CallReply(local_sdp);
                    let msg_str = serde_json::to_string(&msg).unwrap();
                    websocket_send(socket_write.clone(), msg_str).await;
                }

                _ => {}
            }
        }
    }
}
async fn websocket_send(socket_write: Arc<Mutex<SplitSocketWrite>>, msg_str: String) {
    socket_write
        .try_lock()
        .unwrap()
        .send(Text(msg_str))
        .await
        .unwrap();
}
