use std::process;
use std::sync::Arc;

use anyhow::Ok;
use dioxus::desktop::muda::Menu;
use dioxus::desktop::tao::dpi::{PhysicalSize, Size};
use dioxus::desktop::{window, WindowBuilder};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{Mutex, RwLock};

use coupler::Coupler;
use scheduler::{ChannelAttachment, Command, Scheduler, ThreadTypes};
use state::{GUIState, IndependentState};
use utils::Attach;

use dioxus::prelude::*;
use dioxus_logger::tracing::{info, Level};

use crate::app::Chaos;
use crate::peer::Peer;

pub mod app;
pub mod coupler;
pub mod peer;
pub mod scheduler;
pub mod state;
pub mod utils;

const _: &str = manganis::mg!(file("./public/tailwind.css"));
fn main() {
    // let _ = eframe::run_native("Chaos", native_options, Box::new(|_| (Box::new(chaos))));
    dioxus_logger::init(Level::INFO).expect("Failed to initialize dioxus logger.");
    info!("Starting chaos.");
    let cfg = dioxus::desktop::Config::new().with_menu(dioxus::desktop::muda::Menu::new());
    let window = dioxus::desktop::WindowBuilder::new()
        .with_title("chaos")
        .with_min_inner_size(Size::Physical(PhysicalSize {
            width: 720,
            height: 360,
        }));

    LaunchBuilder::desktop()
        .with_cfg(cfg.with_window(window))
        .launch(App);
    //close all threads

    // Ok(())
}
async fn setup_threads() -> ChannelAttachment {
    env_logger::init();
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };
    //setup independent state;
    let independent_state = IndependentState::default();
    let independent_state = Arc::new(RwLock::new(independent_state));

    let mut gui_state = GUIState::default();
    gui_state.display_state = IndependentState::default();

    let independent_state_scheduler = independent_state.clone();
    //setup scheduler
    let mut scheduler = Scheduler::new(independent_state_scheduler); //now scheduler owns independent_state do
                                                                     //not use independent_state directly after
                                                                     //this.

    let scheduler_coupler = crossbeam_channel::unbounded::<Command>();
    let coupler_scheduler = crossbeam_channel::unbounded::<Command>();
    scheduler.attach(
        (scheduler_coupler.0, coupler_scheduler.1),
        Some(ThreadTypes::Coupler),
    );
    let mut coupler = Coupler::new(Arc::new(Mutex::new((
        coupler_scheduler.0,
        scheduler_coupler.1,
    ))))
    .await;
    let scheduler_peer = crossbeam_channel::unbounded::<Command>();
    let peer_scheduler = crossbeam_channel::unbounded::<Command>();

    scheduler.attach(
        (scheduler_peer.0, peer_scheduler.1),
        Some(ThreadTypes::Peer),
    );

    let mut peer = Peer::new(Arc::new(Mutex::new((peer_scheduler.0, scheduler_peer.1)))).await;

    let scheduler_chaos = crossbeam_channel::unbounded::<Command>();
    let chaos_scheduler = crossbeam_channel::unbounded::<Command>();
    scheduler.attach(
        (scheduler_chaos.0, chaos_scheduler.1),
        Some(ThreadTypes::GUI),
    );

    scheduler.run(); //Run scheduler before any other thread
    coupler.start().await;
    peer.start().await;

    let gui_state = Arc::new(Mutex::new(GUIState::default()));
    return (chaos_scheduler.0, scheduler_chaos.1);
    // let chaos = Chaos::new(gui_state, (chaos_scheduler.0, scheduler_chaos.1));
}

#[derive(PartialEq, Props, Clone)]
struct PopupProps {
    tx: Coroutine<Command>,
}
#[component]
fn popup(props: PopupProps) -> Element {
    let mut remote_id = use_signal(|| "".to_string());

    rsx! {
        div {
            class: "bg-[#454545] flex flex-col p-4 h-screen w-full gap-4",
            input {
                class:"bg-[#353535] py-2 px-6 placeholder-[#929292]  rounded-[4px] form-input text-white",
                r#type:"text",
                placeholder: "Enter remote id",
                value: "{remote_id}",
                oninput: move |event| remote_id.set(event.value())
            }
            button {
                class:"py-2 bg-[#566051] text-[#6FC86D] rounded-[4px]",
                onclick: move |_| {props.tx.send(Command::GUI(scheduler::GUICommand::CallRequest(remote_id.to_string())))},
                "Connect"

            }
        }

    }
}
#[component]
fn App() -> Element {
    let tx = use_coroutine(|mut rx: UnboundedReceiver<Command>| async move {
        let attachment = Arc::new(Mutex::new(setup_threads().await));
        while let Some(command) = rx.next().await {
            let attachment = attachment.clone();
            let (tx, _) = attachment.try_lock().unwrap().clone();
            tx.send(command).unwrap();
        }
    });
    let tx_clone = tx.clone();
    rsx! {
        head::Link {
            rel:"stylesheet",
            href: asset!("./assets/main.css")
        }
        div {
            class: "flex flex-row h-screen w-full",
            div {
                class: "flex flex-col w-2/6  bg-[#454545] p-4",
                button {
                    onclick:  move |_| {
                        let dom = VirtualDom::new_with_props(
                            popup, PopupProps { tx: tx_clone.clone()}
                        );
                        let window = dioxus::desktop::WindowBuilder::new()
                        .with_title("connect")
                        .with_max_inner_size(Size::Physical(PhysicalSize {
                            width: 400,
                            height: 200,
                        }));
                        dioxus::desktop::window().new_window(dom, dioxus::desktop::Config::new().with_menu(Menu::new()).with_window(window));
                    },
                    class: "bg-[#566051] px-6 py-2 text-[#6FC86D] rounded-[4px] border-[1px] border-dashed border-[#6FC86D] hover:bg-[#6FC86D] hover:text-[#566051]",
                    "New Chat"
                }
            }
            div {
                class: "flex flex-col bg-[#363636] w-full",

            }

        }

    }
}
