use std::sync::Arc;

use anyhow::Result;
use chrono::Timelike;
use eframe::{Frame, Storage};
use egui::Context;
use egui::WidgetType::TextEdit;
use rand::distributions::Alphanumeric;
use rand::Rng;
use tokio::runtime::{self, Runtime};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::Mutex;
use tokio::time::Duration;
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
use crate::utils::webrtc::CommunicationType;

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
    rx_con: Receiver<CommunicationType>,
    client_id: String,
    remote_client_id: String,
    pub user_messages: Arc<Mutex<Vec<ChaosMessage>>>,
    current_message: String,
    client_sdp: String,
    remote_sdp: String,
}
impl Default for Chaos {
    fn default() -> Self {
        Self {
            tx_gui: channel::<CommunicationType>(32).0,
            rx_con: channel::<CommunicationType>(32).1,
            client_id: rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(4)
                .map(char::from)
                .collect(),
            remote_client_id: Default::default(),
            user_messages: Default::default(),
            current_message: Default::default(),
            client_sdp: Default::default(),
            remote_sdp: Default::default(),
        }
    }
}
impl Chaos {
    pub fn new(
        tx_gui: Sender<CommunicationType>,
        rx_con: Receiver<CommunicationType>,
        u_messages: Arc<Mutex<Vec<ChaosMessage>>>,
    ) -> Self {
        //if let Some(storage) = cc.storage {
        //   return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        //}
        Self {
            tx_gui,
            rx_con,
            user_messages: u_messages,
            ..Default::default()
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
            rx_con,
        } = self;
        //if let Ok(cmd) = rx_con.try_recv() {
        //   use CommunicationType::*;
        //  match cmd {
        //     SendMessage(msg) => {
        //        user_messages.push(ChaosMessage {
        //           client_id: String::from("out"),
        //          message_content: msg,
        //     });
        //}
        // _ => (),
        // }
        //}
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
            ui.label(format!("Your Client SDP is: {client_sdp}"));
            ui.add(egui::TextEdit::singleline(remote_sdp).hint_text("Enter SDP"));
            if ui.button("Connect").clicked() {
                //TODO:  Connect to other clients
                let remote_sdp = remote_sdp.clone();
                let cmd = CommunicationType::MakeConnection(remote_sdp);
                tx_gui.try_send(cmd).unwrap();
            }

            for user_message in user_messages.try_lock().unwrap().iter() {
                ui.label(user_message.client_id.clone() + &*user_message.message_content.clone());
            }
            ui.add(egui::TextEdit::multiline(current_message).hint_text("Enter Message"));

            if ui.button("Send Message").clicked() {
                user_messages.try_lock().unwrap().push(ChaosMessage {
                    message_content: String::from(current_message.as_str()),
                    client_id: String::from(client_id.as_str()),
                });
                current_message.clear();
            }
        });
    }
}
