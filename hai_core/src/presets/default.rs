use std::sync::{Arc, Mutex, MutexGuard};

use crate::{
    node::{Node, NodeLike},
    sprite::Sprite,
    state::State,
};

pub fn add_preset_default<'a>(state: &Arc<Mutex<State<'a>>>) {
    let state = state.lock().unwrap();
    let root_node = state.root_node.clone();
    let mut root_node = root_node.lock().unwrap();
    let root_node = match &mut *root_node {
        NodeLike::Node(n) => n,
        _ => unreachable!("root_node must be a node."),
    };
    let device = state.device.clone();
    let device = device.lock().unwrap();
    let queue = state.queue.clone();
    let queue = queue.lock().unwrap();

    drop(state);

    // load and use texture
    let mut bg = Sprite::from_asset(&device, &queue, "title.png".to_string());
    let mut button1 = Sprite::from_asset(&device, &queue, "button_n_01.png".to_string());
    let mut button2 = Sprite::from_asset(&device, &queue, "button_n_02.png".to_string());
    let mut button3 = Sprite::from_asset(&device, &queue, "button_n_06.png".to_string());

    let mut container = Node::new(
        "Button Container".to_string(),
        Default::default(),
        Default::default(),
    );
    bg.move_to(0, 0);
    container.move_to(923, 0);
    button1.move_to(0, 380);
    button2.move_to(0, 440);
    button3.move_to(0, 560);

    container.add_child(Arc::new(Mutex::new(NodeLike::Sprite(button1))));
    container.add_child(Arc::new(Mutex::new(NodeLike::Sprite(button2))));
    container.add_child(Arc::new(Mutex::new(NodeLike::Sprite(button3))));

    root_node.add_child(Arc::new(Mutex::new(NodeLike::Sprite(bg))));
    root_node.add_child(Arc::new(Mutex::new(NodeLike::Node(container))));
}
