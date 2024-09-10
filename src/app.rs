use std::sync::Arc;

use eframe::Frame;
use egui::Context;
use tokio::sync::Mutex;

use crate::scheduler::{ChannelAttachment, Command, GUICommand};
use crate::state::{ConnectionProgress, GUIState, SidebarButton, UserId};
use crate::utils::Attach;

pub struct Chaos {
    pub gui_state: Arc<Mutex<GUIState>>,
    pub attachment: Arc<Mutex<ChannelAttachment>>,
    // pub independent_state: Arc<RwLock<IndependentState>>, //only for reading
    pub reader_thread: bool,
}
impl Chaos {
    pub fn new(
        gui_state: Arc<Mutex<GUIState>>,
        // independent_state: Arc<RwLock<IndependentState>>,
        attachment: ChannelAttachment,
    ) -> Self {
        Self {
            gui_state,
            // independent_state,
            attachment: Arc::new(Mutex::new(attachment)),
            reader_thread: false,
        }
    }
}
impl Attach for Chaos {
    fn attach(
        &mut self,
        channel_attachment: ChannelAttachment,
        _: Option<crate::scheduler::ThreadTypes>,
    ) {
        self.attachment = Arc::new(Mutex::new(channel_attachment));
    }
}
/*pub struct Chaos {
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
*/

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

/*impl Chaos {
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
}*/
fn menu_bar(ctx: &Context) {
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
}
fn connection_request_popup(
    ctx: &Context,
    remote_id: &UserId,
    attachment: &Arc<Mutex<ChannelAttachment>>,
) {
    ctx.show_viewport_immediate(
        egui::ViewportId::from_hash_of("Connection Request"),
        egui::ViewportBuilder::default()
            .with_title("Connection Request")
            .with_inner_size([200.0, 100.0]),
        |ctx, class| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let attachment = attachment.try_lock().unwrap();
                let (tx, _) = attachment.clone();
                if ui.button("Accept").clicked() {
                    tx.try_send(Command::GUI(GUICommand::CallAnswer(
                        true,
                        remote_id.clone(),
                    )))
                    .unwrap();
                }
                if ui.button("Cancel").clicked() {
                    tx.try_send(Command::GUI(GUICommand::CallAnswer(
                        false,
                        remote_id.clone(),
                    )))
                    .unwrap();
                }
            });
        },
    );
}
fn app_body(
    ctx: &Context,
    gui_state: &mut Arc<Mutex<GUIState>>,
    // independent_state: &mut Arc<RwLock<IndependentState>>,
    attachment: &mut Arc<Mutex<ChannelAttachment>>,
) {
    let gui_state = gui_state.clone();
    let gui_state2 = gui_state.clone();

    // let independent_state = independent_state.clone();
    // let independent_state2 = independent_state.clone(); //never make this mut
    //
    let attachment = attachment.clone();
    let attachment2 = attachment.clone();
    egui::SidePanel::left("sidebar")
        .resizable(true)
        .default_width(150.0)
        .show(ctx, move |ui| {
            let mut gui_state = gui_state.try_lock().unwrap().clone();
            // let independent_state = independent_state.try_read().unwrap();

            if ui
                .selectable_label(
                    gui_state.current_sidebar_button == SidebarButton::NewConnection,
                    format!("{:?}", SidebarButton::NewConnection),
                )
                .clicked()
            {
                gui_state.current_sidebar_button = SidebarButton::NewConnection;
            }
            let attachment = attachment.clone();

            for connection in gui_state.display_state.connections.iter() {
                if connection.1.progress == ConnectionProgress::CallRequestReceived {
                    connection_request_popup(ctx, connection.0, &attachment);
                }
                let sidebar_selected_changed = ui.selectable_label(
                    &gui_state.current_sidebar_button == &SidebarButton::Chat(connection.0.clone()),
                    format!("{:?}", connection.0.clone()),
                );
                if sidebar_selected_changed.clicked() {
                    gui_state.current_sidebar_button = SidebarButton::Chat(connection.0.clone());
                }
            }
        });
    egui::CentralPanel::default().show(ctx, move |ui| {
        let mut gui_state = gui_state2.try_lock().unwrap();
        // let independent_state = independent_state2.try_read().unwrap();
        let attachment = attachment2.clone();

        ui.label(format!(
            "Client ID: {}",
            gui_state.display_state.connection_details.id
        ));
        ui.add(egui::TextEdit::singleline(&mut gui_state.remote_id).hint_text("Enter Remote ID"));
        if ui.button("Connect").clicked() {
            button_click(attachment, gui_state.remote_id.clone());
        }

        if let Some(connection) = gui_state
            .display_state
            .connections
            .get(&gui_state.remote_id)
        {
            ui.label(format!("Connection Progress: {:?}", connection.progress));
        }
    });
}
fn button_click(attachment: Arc<Mutex<ChannelAttachment>>, remote_id: UserId) {
    let (tx, _) = attachment.try_lock().unwrap().clone();
    tx.try_send(Command::GUI(GUICommand::CallRequest(remote_id.clone())))
        .unwrap();
}
impl eframe::App for Chaos {
    //fn save(&mut self, storage: &mut dyn Storage) {
    //    eframe::set_value(storage, eframe::APP_KEY, self);
    //}bin
    fn update(&mut self, ctx: &Context, eframe: &mut Frame) {
        let Self {
            gui_state,
            // independent_state,
            attachment,
            reader_thread,
        } = self;
        menu_bar(ctx);
        app_body(ctx, gui_state, /*independent_state,*/ attachment);
        if !(*reader_thread) {
            *reader_thread = true;
            let ctx = ctx.clone();
            let attachment = attachment.clone();
            let gui_state = gui_state.clone();
            tokio::spawn(async move {
                let (_, rx) = attachment.try_lock().unwrap().clone();
                println!("Starting gui listener");
                let gui_state = gui_state.clone();
                while let Ok(command) = rx.recv() {
                    println!("Received a message on GUI listener.");
                    if let Command::GUI(gui_command) = command {
                        use GUICommand::*;
                        match gui_command {
                            UpdateState(independent_state) => {
                                let mut gui_state = gui_state.try_lock().unwrap();
                                gui_state.display_state = independent_state;
                                ctx.request_repaint();
                                println!("Updated gui state.");
                            }
                            _ => {
                                println!("Not implemented yet");
                            }
                        }
                    }
                }
            });
        }
    }
    /* fn update(&mut self, ctx: &Context, eframe: &mut Frame) {
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
                ui.label(&user_message.message_content);
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
    }*/
}
