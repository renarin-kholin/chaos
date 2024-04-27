use eframe::{Frame, Storage};
use egui::Context;
use egui::WidgetType::TextEdit;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct ChaosMessage{
    client_id: String,
    message_content: String
}
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Chaos{
    client_id: String,
    remote_client_id: String,
    user_messages: Vec<ChaosMessage>,
    current_message: String
}
impl Default for Chaos{
    fn default() -> Self {
        Self{
            client_id: "ABCD".to_string(),
            remote_client_id: Default::default(),
            user_messages: Default::default(),
            current_message: Default::default()
        }
    }
}
impl Chaos{
    pub fn new(cc: &eframe::CreationContext<'_>) ->Self{
        if let Some(storage) = cc.storage{
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }
        Default::default()
    }
}
impl eframe::App for Chaos{
    fn save(&mut self, storage: &mut dyn Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
    fn update(&mut self, ctx: &Context, eframe: &mut Frame) {
        let Self{
            remote_client_id,
            client_id,
            user_messages,
            current_message
        } = self;
        egui::TopBottomPanel::top("top_pannel").show(ctx, |ui|{
           egui::menu::bar(ui, |ui|{
              ui.menu_button("File", |ui|{
                  if ui.button("Quit").clicked(){
                      ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                  }
              });
               ui.add_space(16.0);
               egui::widgets::global_dark_light_mode_buttons(ui);
           });
        });
        egui::CentralPanel::default().show(ctx, |ui|{
            ui.add(egui::TextEdit::singleline(remote_client_id).hint_text("Enter Client ID"));
            if ui.button("Connect").clicked(){
                //TODO: Connect to other clients
            }

            for user_message in user_messages.iter() {

                ui.label(user_message.client_id.clone() + &*user_message.message_content.clone());

            }
            ui.add(egui::TextEdit::multiline(current_message).hint_text("Enter Message"));
            
            if ui.button("Send Message").clicked(){
                user_messages.push(ChaosMessage { message_content: String::from(current_message.as_str()), client_id: String::from(client_id.as_str()) });
                current_message.clear();
            }
        });
    }


}