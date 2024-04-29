use std::sync::{
    mpsc::{self, channel},
    Arc,
};

use anyhow::Result;

use tokio::{sync::Mutex, time::Duration};
use webrtc::{
    api::{
        interceptor_registry::register_default_interceptors, media_engine::MediaEngine, APIBuilder,
    },
    data_channel::{data_channel_message::DataChannelMessage, RTCDataChannel},
    ice_transport::{ice_credential_type::RTCIceCredentialType, ice_server::RTCIceServer},
    interceptor::registry::Registry,
    peer_connection::{
        configuration::RTCConfiguration, math_rand_alpha,
        peer_connection_state::RTCPeerConnectionState,
        sdp::session_description::RTCSessionDescription,
    },
};

use crate::app::{Chaos, ChaosMessage};
mod app;
pub mod utils;
use utils::{crypto, webrtc::CommunicationType};

#[tokio::main]
async fn main() -> eframe::Result<()> {
    env_logger::init();
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };
    let (tx_gui, mut rx_gui) = tokio::sync::mpsc::channel::<CommunicationType>(32);
    let (tx_con, mut rx_con) = tokio::sync::mpsc::channel::<CommunicationType>(32);
    let u_messages: Arc<Mutex<Vec<ChaosMessage>>> = Arc::new(Mutex::new(Vec::new()));
    let mut chaos = Chaos::new(tx_gui, rx_con, u_messages.clone());
    let um_clone = Arc::clone(&u_messages);
    tokio::spawn(async move {
        if let Some(cmd) = rx_gui.recv().await {
            use CommunicationType::*;
            match cmd {
                MakeConnection(remote_sdp) => {
                    let mut m = MediaEngine::default();
                    m.register_default_codecs();
                    let mut registry = Registry::new();
                    registry = register_default_interceptors(registry, &mut m).unwrap();

                    let api = APIBuilder::new()
                        .with_media_engine(m)
                        .with_interceptor_registry(registry)
                        .build();
                    let config = RTCConfiguration {
                        ice_servers: vec![
                            RTCIceServer {
                                urls: vec!["stun:stun.l.google.com:19302".to_owned()],
                                ..Default::default()
                            },
                            RTCIceServer {
                                urls: vec!["turn:global.relay.metered.ca".to_owned()],
                                username: "46c967b98ec9994d702767e8".to_owned(),
                                credential: "lcz6Ykhd14hYyKfP".to_owned(),
                                credential_type: RTCIceCredentialType::Password,
                            },
                        ],

                        ..Default::default()
                    };
                    let peer_connection = Arc::new(api.new_peer_connection(config).await.unwrap());
                    let (done_tx, mut done_rx) = tokio::sync::mpsc::channel::<u8>(1);

                    peer_connection.on_peer_connection_state_change(Box::new(
                        move |s: RTCPeerConnectionState| {
                            println!("Peer Connection State has changed: {s}");
                            if s == RTCPeerConnectionState::Failed {
                                println!("Peer Connection has gone to failed exiting");
                                let _ = done_tx.try_send(1);
                            }
                            Box::pin(async {})
                        },
                    ));
                    peer_connection.on_data_channel(Box::new(move |d: Arc<RTCDataChannel>| {
                        let d_label = d.label().to_owned();
                        let d_id = d.id();
                        println!("New DataChannel {d_label} {d_id}");
                        let (tx_con2, mut rx_con2) =
                            tokio::sync::mpsc::channel::<CommunicationType>(32);
                        let um_clone = um_clone.clone();

                        Box::pin(async move {
                            let d2 = Arc::clone(&d);
                            let d_label2 = d_label.clone();
                            let d_id2 = d_id;
                            let um_clone = um_clone.clone();

                            d.on_close(Box::new(move || {
                                println!("Data Channel closed");
                                Box::pin(async {})
                            }));
                            d.on_open(Box::new(move || {
                                println!("Data Channel '{d_label2}'-'{d_id2}' open.");
                                Box::pin(async move {
                                    let mut result = Result::<usize>::Ok(0);
                                    while result.is_ok() {
                                        let timeout = tokio::time::sleep(Duration::from_secs(5));
                                        tokio::pin!(timeout);

                                        tokio::select! {
                                         _ = timeout.as_mut() =>{
                                         let message = math_rand_alpha(15);
                                         println!("Sending '{message}'");
                                         result = d2.send_text(message).await.map_err(Into::into);
                                         }
                                        };
                                    }
                                })
                            }));

                            d.on_message(Box::new(move |msg: DataChannelMessage| {
                                let msg_str = String::from_utf8(msg.data.to_vec()).unwrap();
                                println!("Message from data_channel '{msg_str}'");
                                um_clone.try_lock().unwrap().push(ChaosMessage {
                                    client_id: String::from("test"),
                                    message_content: msg_str,
                                });
                                //test_tx.try_send(1);

                                Box::pin(async {
                                    // tx_con.send(cmd).await.unwrap();
                                })
                            }));
                        })
                    }));
                    let desc_data = crypto::decode_b64(&remote_sdp).unwrap();
                    let offer = serde_json::from_str::<RTCSessionDescription>(&desc_data).unwrap();
                    peer_connection.set_remote_description(offer).await.unwrap();
                    let answer = peer_connection.create_answer(None).await.unwrap();
                    let mut gather_complete = peer_connection.gathering_complete_promise().await;
                    peer_connection.set_local_description(answer).await.unwrap();
                    let _ = gather_complete.recv().await;
                    if let Some(local_desc) = peer_connection.local_description().await {
                        let json_str = serde_json::to_string(&local_desc).unwrap();
                        let answerb64 = crypto::encode_b64(&json_str);
                        println!("{answerb64}");
                    } else {
                        println!("Generate local_description failed");
                    }
                }
                _ => (),
            }
        }
    });
    eframe::run_native("chaos", native_options, Box::new(|_| (Box::new(chaos))))
}
