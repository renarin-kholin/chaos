use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::{Mutex, RwLock};

use coupler::Coupler;
use scheduler::{Command, Scheduler, ThreadTypes};
use state::{GUIState, IndependentState};
use utils::Attach;

use crate::app::Chaos;
use crate::peer::Peer;

pub mod app;
pub mod coupler;
pub mod peer;
pub mod scheduler;
pub mod state;
pub mod utils;

#[tokio::main]
async fn main() -> eframe::Result<()> {
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

    let chaos = Chaos::new(gui_state, (chaos_scheduler.0, scheduler_chaos.1));

    let _ = eframe::run_native("Chaos", native_options, Box::new(|_| (Box::new(chaos))));
    //setup coupler thread
    Ok(())
}
