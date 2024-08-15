use std::sync::Arc;

use tokio::sync::Mutex;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::{APIBuilder, API};
use webrtc::data::data_channel;
use webrtc::data_channel::RTCDataChannel;
use webrtc::ice_transport::ice_credential_type::RTCIceCredentialType;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

use crate::scheduler::{ChannelAttachment, Command, PeerCommand};
use crate::utils::crypto;

pub struct Peer {
    pub attachment: Arc<Mutex<ChannelAttachment>>,
    pub rtc_config: RTCConfiguration,
    pub rtc_api: API,
}
impl Peer {
    pub async fn new(attachment: Arc<Mutex<ChannelAttachment>>) -> Self {
        let mut media_engine = MediaEngine::default();
        media_engine
            .register_default_codecs()
            .expect("Could not register default codecs.");
        let mut registry = Registry::new();
        registry = register_default_interceptors(registry, &mut media_engine).unwrap();

        let rtc_api = APIBuilder::new()
            .with_media_engine(media_engine)
            .with_interceptor_registry(registry)
            .build();
        let rtc_config = RTCConfiguration {
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
        Self {
            attachment,
            rtc_config,
            rtc_api,
        }
    }
    pub async fn start(&mut self) {
        let attachment = self.attachment.clone();

        let peer_connection = Arc::new(
            self.rtc_api
                .new_peer_connection(self.rtc_config.clone())
                .await
                .unwrap(),
        );
        let data_channel = peer_connection
            .create_data_channel("data/userid", None)
            .await
            .unwrap();
        peer_connection.on_peer_connection_state_change(Box::new(
            move |s: RTCPeerConnectionState| {
                println!("Peer Connection state has changed: {s}");
                if s == RTCPeerConnectionState::Failed {
                    println!("Peer Connection has changed state to failed. exiting..");
                }
                Box::pin(async {})
            },
        ));
        data_channel.on_open(Box::new(move || {
            println!("Data channel is now open.");
            Box::pin(async {})
        }));
        let peer_connection = Arc::new(Mutex::new(peer_connection));
        let data_channel = Arc::clone(&data_channel);

        tokio::spawn(async move {
            let attachment = attachment.clone();
            let (_, rx) = attachment.try_lock().unwrap().clone();
            while let Ok(command) = rx.recv() {
                if let Command::Peer(command) = command {
                    match command {
                        PeerCommand::NewPeerConnection(remote_id) => {
                            let peer_connection = peer_connection.try_lock().unwrap();
                            let offer = peer_connection.create_offer(None).await.unwrap();
                            let mut gather_complete =
                                peer_connection.gathering_complete_promise().await;
                            let _ = peer_connection.set_local_description(offer).await;
                            let _ = gather_complete.recv().await;
                            if let Some(local_description) =
                                peer_connection.local_description().await
                            {
                                let local_description_string =
                                    serde_json::to_string(&local_description).unwrap();
                                let encrypted_local_description_string =
                                    crypto::encode_b64(&local_description_string);

                                let (tx, _) = attachment.try_lock().unwrap().clone();
                                tx.try_send(Command::Peer(PeerCommand::CallAnswer(
                                    encrypted_local_description_string,
                                )))
                                .unwrap();
                            }
                        }
                        PeerCommand::EstablishConnection(remote_sdp, is_reply) => {
                            let peer_connection = peer_connection.try_lock().unwrap();
                            let remote_description = crypto::decode_b64(&remote_sdp).unwrap();
                            let remote_offer =
                                serde_json::from_str::<RTCSessionDescription>(&remote_description)
                                    .unwrap();
                            peer_connection
                                .set_remote_description(remote_offer)
                                .await
                                .unwrap();
                            if is_reply {
                                break;
                            }
                            if !is_reply {
                                let answer = peer_connection.create_answer(None).await.unwrap();
                                let mut gather_complete =
                                    peer_connection.gathering_complete_promise().await;
                                peer_connection.set_local_description(answer).await.unwrap();
                                let _ = gather_complete.recv().await;

                                if let Some(local_description) =
                                    peer_connection.local_description().await
                                {
                                    let local_description_string =
                                        serde_json::to_string(&local_description).unwrap();
                                    let encrypted_local_description_string =
                                        crypto::encode_b64(&local_description_string);

                                    let (tx, _) = attachment.try_lock().unwrap().clone();
                                    tx.try_send(Command::Peer(PeerCommand::CallReply(
                                        encrypted_local_description_string,
                                    )))
                                    .unwrap();
                                }
                            }
                        }
                        _ => {
                            println!("Not implemented yet.");
                        }
                    }
                }
            }
        });
    }
}
