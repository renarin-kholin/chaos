use std::sync::{
    mpsc::{self, channel},
    Arc,
};

use anyhow::Result;

use futures_util::{SinkExt, StreamExt};
use tokio::{
    sync::{Mutex, RwLock},
    time::Duration,
};
use tokio_tungstenite::{connect_async, tungstenite::Message};
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
use url::Url;
mod app;
pub mod utils;
use utils::{
    crypto,
    message::{CCErrors, CCMessage},
    webrtc::CommunicationType,
};

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
    let u_messages: Arc<RwLock<Vec<ChaosMessage>>> = Arc::new(RwLock::new(Vec::new()));
    let tx_gui_clone = tx_gui.clone();

    let (ws_stream, _) = connect_async(Url::parse("ws://localhost:3030/couple").unwrap())
        .await
        .expect("Can't connect to the websocket server");
    println!("Connected to the coupling websockets server.");
    let (mut write, mut read) = ws_stream.split();

    let mut socket = Arc::new(Mutex::new(write));
    let mut socket_clone = Arc::clone(&socket);
    let mut socket_clone2 = Arc::clone(&socket);

    let mut connection_errors = Arc::new(Mutex::new(CCErrors::default()));
    let mut connection_errors_clone = Arc::clone(&connection_errors);
    let mut call_request = Arc::new(Mutex::new(false));
    let mut call_request_clone = Arc::clone(&call_request);

    let mut chaos = Chaos::new(
        tx_gui,
        tx_con,
        u_messages.clone(),
        socket_clone2,
        connection_errors_clone,
        call_request_clone,
    );

    let mut client_id = Arc::clone(&chaos.client_id);
    let mut connection_errors_clone2 = Arc::clone(&connection_errors);
    let mut call_request_clone = Arc::clone(&call_request);
    let (tx_agree, mut rx_agree) = tokio::sync::mpsc::channel::<String>(32);
    let tx_agree = Arc::new(Mutex::new(tx_agree));
    let tx_agree = Arc::clone(&tx_agree);
    let mut rx_agree = Arc::new(Mutex::new(rx_agree));
    let mut rx_agree = Arc::clone(&rx_agree);
    tokio::spawn(async move {
        while let Some(msg) = read.next().await {
            if let Ok(cc_message) = serde_json::from_str::<CCMessage>(&msg.unwrap().to_string()) {
                println!("Received a CCMessage {:?}", cc_message);
                match cc_message {
                    CCMessage::SetClientId(cid) => {
                        let mut client_id = client_id.try_lock().unwrap();
                        *client_id = cid;
                    }
                    CCMessage::CallRequestFailure => {
                        let mut connection_errors = connection_errors_clone2.try_lock().unwrap();
                        *connection_errors = CCErrors::CallRequestFailure;
                    }
                    CCMessage::CallRequest(rid) => {
                        //Ask the user if they want to accept the call;
                        let mut call_request = call_request_clone.try_lock().unwrap();
                        *call_request = true;
                    }
                    CCMessage::CallAnswer(agreed, sdp) => {
                        if agreed {
                            tx_gui_clone.try_send(CommunicationType::MakeConnection(sdp.unwrap()));
                        }
                    }
                    CCMessage::CallReply(sdp) => {
                        tx_agree.try_lock().unwrap().try_send(sdp);
                    }

                    _ => (),
                }
            };
        }
    });

    let mut client_sdp = Arc::clone(&chaos.client_sdp);

    let rx_con = Arc::new(Mutex::new(rx_con));
    let rx_con_clone = Arc::clone(&rx_con);

    let um_clone = Arc::clone(&u_messages);
    let rx_gui = Arc::new(Mutex::new(rx_gui));
    let rx_gui_clone = Arc::clone(&rx_gui);

    tokio::spawn(async move {
        if let Some(cmd) = rx_gui_clone.try_lock().unwrap().recv().await {
            use CommunicationType::*;
            match cmd {
                SendCCMessage(CCMessage::CallAnswer(agreed, _)) => {
                    let mut socket = Arc::clone(&socket);
                    if agreed {
                        let mut m = MediaEngine::default();
                        m.register_default_codecs();
                        let mut registry = Registry::new();
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
                        let peer_connection =
                            Arc::new(api.new_peer_connection(config).await.unwrap());
                        let data_channel = peer_connection
                            .create_data_channel("data", None)
                            .await
                            .unwrap();
                        let p1 = Arc::clone(&peer_connection);

                        peer_connection.on_peer_connection_state_change(Box::new(
                            move |s: RTCPeerConnectionState| {
                                println!("Peer connection state has changed: {s}");
                                if s == RTCPeerConnectionState::Failed {
                                    println!("Peer Connection has gone to failed exiting");
                                }
                                Box::pin(async {})
                            },
                        ));
                        let d1 = Arc::clone(&data_channel);
                        data_channel.on_open(Box::new(move || {
                            println!("Data channel is now open");
                            let d2 = Arc::clone(&d1);
                            Box::pin(async {})
                        }));
                        let d_label = data_channel.label().to_owned();
                        let d2 = Arc::clone(&data_channel);

                        tokio::spawn(async move {
                            while let Some(cmd) = rx_con_clone.lock().await.recv().await {
                                match cmd {
                                    CommunicationType::SendMessage(msg) => {
                                        d2.send_text(msg.message_content).await;
                                    }
                                    _ => (),
                                }
                            }
                        });
                        data_channel.on_message(Box::new(move |msg: DataChannelMessage| {
                            let msg_str = String::from_utf8(msg.data.to_vec()).unwrap();
                            println!("Message from datachannel: {}", msg_str);

                            println!("Message from data_channel '{msg_str}'");
                            um_clone.try_write().unwrap().push(ChaosMessage {
                                client_id: String::from("test"),
                                message_content: msg_str,
                            });
                            //test_tx.try_send(1);
                            Box::pin(async {})
                        }));
                        let offer = peer_connection.create_offer(None).await.unwrap();
                        let mut gather_complete =
                            peer_connection.gathering_complete_promise().await;
                        peer_connection.set_local_description(offer).await;
                        let _ = gather_complete.recv().await;
                        if let Some(local_desc) = peer_connection.local_description().await {
                            let json_str = serde_json::to_string(&local_desc).unwrap();
                            let b64 = crypto::encode_b64(&json_str);

                            let cc_message = CCMessage::CallAnswer(true, Some(b64));
                            let msg_str = serde_json::to_string(&cc_message).unwrap();
                            let mut socket = socket.try_lock().unwrap();
                            socket.send(Message::Text(msg_str)).await;
                        } else {
                            println!("Generate local description failed!");
                        }
                        tokio::spawn(async move {
                            if let Some(msg) = rx_agree.try_lock().unwrap().recv().await {
                                let p2 = Arc::clone(&p1);
                                let desc_data = crypto::decode_b64(&msg).unwrap();
                                let answer =
                                    serde_json::from_str::<RTCSessionDescription>(&desc_data)
                                        .unwrap();
                                p2.set_remote_description(answer).await.unwrap();
                            }
                        });
                    } else {
                        let cc_message = CCMessage::CallAnswer(false, None);
                        let msg_str = serde_json::to_string(&cc_message).unwrap();
                        let mut socket = socket.try_lock().unwrap();
                        socket.send(Message::Text(msg_str)).await;
                    }
                }
                MakeConnection(remote_sdp) => {
                    let mut m = MediaEngine::default();
                    let mut socket = Arc::clone(&socket);
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
                        let um_clone = um_clone.clone();
                        let rx_con_clone = rx_con_clone.clone();
                        let shared_dc = d.clone();
                        tokio::spawn(async move {
                            while let Some(cmd) = rx_con_clone.lock().await.recv().await {
                                match cmd {
                                    CommunicationType::SendMessage(msg) => {
                                        shared_dc.send_text(msg.message_content).await;
                                    }
                                    _ => (),
                                }
                            }
                        });

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
                                Box::pin(async move {})
                            }));

                            d.on_message(Box::new(move |msg: DataChannelMessage| {
                                let msg_str = String::from_utf8(msg.data.to_vec()).unwrap();
                                println!("Message from data_channel '{msg_str}'");
                                um_clone.try_write().unwrap().push(ChaosMessage {
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
                        let mut client_sdp = client_sdp.try_lock().unwrap();
                        *client_sdp = answerb64.clone(); //Make it so that SDPs are shared automatically
                                                         //using websockets
                        let msg = CCMessage::CallReply(answerb64);
                        let msg_str = serde_json::to_string(&msg).unwrap();
                        let mut socket = socket.try_lock().unwrap();
                        socket.send(Message::Text(msg_str)).await.unwrap();
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
