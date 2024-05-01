use std::sync::Arc;

use anyhow::Result;
use chrono::Timelike;
use eframe::{Frame, Storage};
use egui::WidgetType::TextEdit;
use egui::{Color32, Context};
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, TryFutureExt};
use rand::distributions::Alphanumeric;
use rand::Rng;
use tokio::net::TcpStream;
use tokio::runtime::{self, Runtime};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::{Mutex, RwLock};
use tokio::time::Duration;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::data_channel::RTCDataChannel;
use webrtc::ice_transport::ice_credential_type::RTCIceCredentialType;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::math_rand_alpha;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

use crate::utils::crypto;
use crate::utils::message::{CCErrors, CCMessage};
use crate::utils::webrtc::CommunicationType;

#[derive(Clone)]
pub struct ChaosMessage {
    pub client_id: String,
    pub message_content: String,
}
impl Default for ChaosMessage {
    fn default() -> Self {
        Self {
            client_id: Default::default(),
            message_content: Default::default(),
        }
    }
}

pub struct Chaos {
    tx_gui: Sender<CommunicationType>,
    tx_con: Sender<CommunicationType>,
    pub client_id: Arc<Mutex<String>>,
    remote_client_id: String,
    pub user_messages: Arc<RwLock<Vec<ChaosMessage>>>,
    current_message: String,
    pub client_sdp: Arc<Mutex<String>>,
    remote_sdp: String,
    pub socket: Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>,
    pub connection_errors: Arc<Mutex<CCErrors>>,
    pub call_request: Arc<Mutex<bool>>,
}
//impl Default for Chaos {
//   fn default() -> Self {
//  Self {
//      tx_gui: channel::<CommunicationType>(32).0,
//      tx_con: channel::<CommunicationType>(32).0,
//       client_id: Default::default(),
//        remote_client_id: Default::default(),
//         user_messages: Default::default(),
//        current_message: Default::default(),
//         client_sdp: Default::default(),
//         remote_sdp: Default::default(),
//         socket: Arc::new(Mutex::new())
//      }
//   }
//}
//move modifiable state to a single struct
impl Chaos {
    pub fn new(
        tx_gui: Sender<CommunicationType>,
        tx_con: Sender<CommunicationType>,
        u_messages: Arc<RwLock<Vec<ChaosMessage>>>,
        socket: Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>,
        connection_errors: Arc<Mutex<CCErrors>>,
        call_request: Arc<Mutex<bool>>,
    ) -> Self {
        //if let Some(storage) = cc.storage {
        //   return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        //}
        Self {
            tx_gui,
            tx_con,
            user_messages: u_messages,
            socket,
            client_id: Default::default(),
            remote_client_id: Default::default(),
            current_message: Default::default(),
            client_sdp: Default::default(),
            remote_sdp: Default::default(),
            connection_errors,
            call_request,
        }
    }
}
impl eframe::App for Chaos {
    //fn save(&mut self, storage: &mut dyn Storage) {
    //    eframe::set_value(storage, eframe::APP_KEY, self);
    //}
    fn update(&mut self, ctx: &Context, eframe: &mut Frame) {
        let Self {
            remote_client_id,
            client_id,
            user_messages,
            current_message,
            client_sdp,
            remote_sdp,
            tx_gui,
            tx_con,
            socket,
            connection_errors,
            call_request,
        } = self;

        egui::TopBottomPanel::top("top_pannel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.add_space(16.0);
                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(format!(
                "Your Client ID is: {}",
                client_id.try_lock().unwrap()
            ));
            ui.add(egui::TextEdit::singleline(remote_client_id).hint_text("Enter Remote ID"));
            match *connection_errors.clone().try_lock().unwrap() {
                CCErrors::CallRequestFailure => {
                    ui.colored_label(
                        Color32::from_rgb(209, 86, 94),
                        format!(
                            "Call failed, User with the Id: {} is not connected.",
                            remote_client_id
                        ),
                    );
                }
                CCErrors::None => (),
            }
            if *call_request.try_lock().unwrap(){
                
                ctx.show_viewport_immediate(
                    egui::ViewportId::from_hash_of("immediate_viewport"),
                    egui::ViewportBuilder::default().with_title("Call Request").with_inner_size([200.0, 100.0]), |ctx, class| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                    ui.label(format!("You have received a call request from: {}. Would you like to accept it?", remote_client_id));
                    if ui.button("Agree").clicked() {

                       *call_request.try_lock().unwrap() = false; 
                       //Send answer
                       let msg  = CommunicationType::SendCCMessage(CCMessage::CallAnswer(true, None) );
                        
                       tx_gui.try_send(msg).unwrap();
                       
                    }
                   if  ui.button("Cancel").clicked() {

                       *call_request.try_lock().unwrap() = false;

                    }
                        });

                });
                
            } 
            if ui.button("Connect").clicked() {
                //TODO:  Connect to other clients
                let mut connection_errors = connection_errors.try_lock().unwrap();
                *connection_errors = CCErrors::None;

                let remote_client_id = remote_client_id.clone();
                let cmd = CCMessage::CallRequest(remote_client_id);
                let cmd_str = serde_json::to_string(&cmd).unwrap();
                println!("Sending CCMEssage ");
                let socket_clone = socket.clone();
                tokio::task::spawn(async move {
                    socket_clone
                        .try_lock()
                        .unwrap()
                        .send(Message::Text(cmd_str))
                        .await
                        .unwrap_or_else(|e| println!("sned socket error {}", e));
                });

                //socket.unwrap().send(tungstenite::Message::Text(cmd_str));
            }

            for user_message in user_messages.try_read().unwrap().iter() {
                ui.label(user_message.client_id.clone() + &*user_message.message_content.clone());
            }
            ui.add(egui::TextEdit::multiline(current_message).hint_text("Enter Message"));

            if ui.button("Send Message").clicked() {
                let message = ChaosMessage {
                    message_content: String::from(current_message.as_str()),
                    client_id: String::from(client_id.try_lock().unwrap().as_str()),
                };
                user_messages.try_write().unwrap().push(message.clone());
                tx_con.try_send(CommunicationType::SendMessage(message));
                current_message.clear();
            }
        });
    }
}
