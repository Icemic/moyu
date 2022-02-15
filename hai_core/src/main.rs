#![feature(drain_filter)]

mod node;
mod renderer;
mod sprite;
mod texture;
mod traits;
mod types;

#[cfg(not(target_arch = "wasm32"))]
use hai_js_runtime::JSRuntime;
use hai_pal::{env, logger, platform};
use node::{Node, NodeLike};
use renderer::Renderer;
use sprite::Sprite;
#[cfg(not(target_arch = "wasm32"))]
use std::thread;
use winit::{
    dpi::{LogicalSize, Size},
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() {
    platform::setup();
    env::setup();
    logger::setup();

    // init v8
    #[cfg(not(target_arch = "wasm32"))]
    {
        thread::spawn(|| {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async {
                let mut vm = JSRuntime::new();
                vm.prepare_static_modules().await;
                vm.start();

                vm.run_event_loop().await;
            });
        });
    }

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_inner_size(Size::Logical(LogicalSize::new(1280., 720.)))
        .build(&event_loop)
        .unwrap();

    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                body.append_child(&web_sys::Element::from(window.canvas()))
                    .ok()
            })
            .expect("couldn't append canvas to document body");
    }

    #[cfg(not(target_arch = "wasm32"))]
    let mut renderer = {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        runtime.block_on(Renderer::new(&window))
    };

    #[cfg(target_arch = "wasm32")]
    let mut renderer = { pollster::block_on(Renderer::new(&window)) };

    // load and use texture
    let mut bg = Sprite::from_asset(&renderer, "title.png");
    let mut button1 = Sprite::from_asset(&renderer, "button_n_01.png");
    let mut button2 = Sprite::from_asset(&renderer, "button_n_02.png");
    let mut button3 = Sprite::from_asset(&renderer, "button_n_06.png");

    let mut container = Node::new(
        Some("Button Container"),
        Default::default(),
        Default::default(),
    );
    bg.move_to(0, 0);
    container.move_to(923, 0);
    button1.move_to(0, 380);
    button2.move_to(0, 440);
    button3.move_to(0, 560);

    container.add_child(NodeLike::Sprite(button1));
    container.add_child(NodeLike::Sprite(button2));
    container.add_child(NodeLike::Sprite(button3));

    renderer.get_root_node().add_child(NodeLike::Sprite(bg));
    renderer
        .get_root_node()
        .add_child(NodeLike::Node(container));

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                renderer.update();
                match renderer.render() {
                    Ok(_) => {}
                    // Reconfigure the surface if lost
                    Err(wgpu::SurfaceError::Lost) => renderer.refresh(),
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                window.request_redraw();
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                // makes State to have priority over main()
                if !renderer.input(event) {
                    // UPDATED!
                    match event {
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(physical_size) => {
                            renderer.resize(*physical_size, None);
                        }
                        WindowEvent::ScaleFactorChanged {
                            scale_factor,
                            new_inner_size,
                            ..
                        } => {
                            renderer.resize(**new_inner_size, Some(*scale_factor));
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    });
}
