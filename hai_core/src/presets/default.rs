use hai_pal::sync::RwLock;
use std::sync::Arc;

use crate::{
    nodes::{Container, Sprite},
    state::State,
    traits::Node,
};

pub fn add_preset_default(state: &Arc<RwLock<State>>) {
    let state = state.read();
    let root_node = state.root_node.clone();
    let mut root_node = root_node.lock();
    let device = state.device.clone();
    let device = device.lock();
    let queue = state.queue.clone();
    let queue = queue.lock();

    drop(state);

    // // load and use texture
    // let mut bg = Sprite::new(&device, &queue, "title.png".to_string());
    // let mut button1 = Sprite::new(&device, &queue, "button_n_01.png".to_string());
    // let mut button2 = Sprite::new(&device, &queue, "button_n_02.png".to_string());
    // let mut button3 = Sprite::new(&device, &queue, "button_n_06.png".to_string());

    // let mut container = Container::new(
    //     "Button Container".to_string(),
    //     Default::default(),
    //     Default::default(),
    // );
    // bg.move_to(0, 0);
    // container.move_to(923, 0);
    // button1.move_to(0, 380);
    // button2.move_to(0, 440);
    // button3.move_to(0, 560);

    // container.add_child(Arc::new(Mutex::new(button1)));
    // container.add_child(Arc::new(Mutex::new(button2)));
    // container.add_child(Arc::new(Mutex::new(button3)));

    // root_node.add_child(Arc::new(Mutex::new(bg)));
    // root_node.add_child(Arc::new(Mutex::new(container)));
}
