//! Shared engine assembly.
//!
//! The embedding application owns the platform: the window system, the graphics device
//! and the frame loop. Everything else a Moyu runtime needs to run a project is
//! assembled here — the built-in nodes and their renderers, the plugin set, the script
//! runtime and the splash screen. The default windowed runtime (`crates/moyu`) and the
//! Godot extension (`crates/godot`) both go through this module, so their startup orders
//! stay in step.

mod splash;

pub use splash::show_splash_screen;

use std::sync::Arc;

use moyu_audio::AudioManager;
use moyu_core::core::Core;
use moyu_core::events::GameEvent;
use moyu_core::utils::dispatch_event::dispatch_event;
use moyu_gamepad::GamepadPlugin;
#[cfg(feature = "text")]
use moyu_nodes::nodes::Text;
use moyu_nodes::nodes::{
    Animation, Backdrop, Clip, Editable, Filter, HBox, Shader, ShaderSlot, Sprite, VBox, Video,
};
#[cfg(feature = "text")]
use moyu_nodes::renderer::TextRenderer;
use moyu_nodes::renderer::{
    AnimationRenderer, BackdropRenderer, ClipRenderer, OffscreenPassRenderer, ShaderRenderer,
    ShaderSlotRenderer, SpriteRenderer, VideoRenderer,
};
use moyu_pal::config::get_engine_config;
use moyu_pal::sync::Mutex;
use moyu_scenario::ScenarioPlugin;

#[cfg(native)]
pub use moyu_pal::visible_hand::VisibleHand;

/// Handle that keeps the script runtime alive for as long as the engine runs.
#[cfg(native)]
pub type ScriptRuntime = VisibleHand<Arc<moyu_runtime::QuickVM>>;
/// The web runtime belongs to the page, so there is no handle to keep.
#[cfg(web)]
pub type ScriptRuntime = ();

/// Register the built-in node types and their renderers.
///
/// Graphics must be initialized: the renderers are built against the frame output's
/// texture format and sample count.
pub fn register_builtins(core: &Arc<Core>) {
    core.register_node_type::<moyu_core::nodes::Container>("container");
    core.register_node_type::<VBox>("vbox");
    core.register_node_type::<HBox>("hbox");
    core.register_node_type::<Sprite>("sprite");
    #[cfg(feature = "text")]
    core.register_node_type::<Text>("text");
    core.register_node_type::<Clip>("clip");
    core.register_node_type::<Editable>("editable");
    core.register_node_type::<Filter>("filter");
    core.register_node_type::<Backdrop>("backdrop");
    core.register_node_type::<Animation>("animation");
    core.register_node_type::<Shader>("shader");
    core.register_node_type::<ShaderSlot>("shader-slot");
    core.register_node_type::<Video>("video");

    let Some(graphics) = core.graphics() else {
        log::warn!("graphics is not initialized, built-in nodes have no renderers");
        return;
    };

    let device = graphics.device();
    let format = graphics.output_format();
    let sample_count = graphics.sample_count();

    graphics.register_renderer(
        "sprite",
        Box::new(SpriteRenderer::new(device, format, sample_count)),
    );
    graphics.register_renderer("clip", Box::new(ClipRenderer::new(device, format)));
    graphics.register_renderer(
        "filter",
        Box::new(OffscreenPassRenderer::new(device, format, sample_count)),
    );
    graphics.register_renderer(
        "backdrop",
        Box::new(BackdropRenderer::new(device, format, sample_count)),
    );
    graphics.register_renderer(
        "animation",
        Box::new(AnimationRenderer::new(device, format, sample_count)),
    );
    graphics.register_renderer(
        "shader",
        Box::new(ShaderRenderer::new(device, format, sample_count)),
    );
    graphics.register_renderer("shader-slot", Box::new(ShaderSlotRenderer::new()));
    graphics.register_renderer(
        "video",
        Box::new(VideoRenderer::new(device, format, sample_count)),
    );

    #[cfg(feature = "text")]
    {
        let text_renderer = TextRenderer::new(device, format, sample_count);
        text_renderer.init_huozi_from_env();
        graphics.register_renderer("text", Box::new(text_renderer));
    }
}

/// Register the plugin set: audio, system, scenario and, when the project enables
/// them, gamepads. Returns once every plugin is registered and ready.
pub async fn register_plugins(core: &Arc<Core>) {
    match AudioManager::new() {
        Ok(audio_manager) => {
            core.register_plugin("audio", Arc::new(Mutex::new(audio_manager)));
        }
        Err(err) => {
            log::error!("failed to create audio manager: {}", err);
        }
    }

    let system = moyu_core::plugins::SystemPlugin::new(core.clone());
    core.register_plugin("system", Arc::new(Mutex::new(system)));

    let mut scenario = ScenarioPlugin::new();
    if let Err(err) = scenario.init().await {
        log::error!("Failed to initialize scenario plugin: {}", err);
    }
    core.register_plugin("scenario", Arc::new(Mutex::new(scenario)));

    #[cfg(any(desktop, web))]
    if get_engine_config().enable_gamepads {
        log::info!("enable gamepad plugin");
        let gamepad = GamepadPlugin::new();
        core.register_plugin("gamepad", Arc::new(Mutex::new(gamepad)));
    }
}

/// Load the project entry script. `on_load` runs once the script has loaded, which is
/// when the project's own content exists and the first frame can be drawn.
pub fn load_user_script<F>(core: &Arc<Core>, on_load: F) -> anyhow::Result<ScriptRuntime>
where
    F: FnOnce() + Send + 'static,
{
    moyu_ops::spawn::spawn_runtime_with_core(core, on_load)
}

/// Tell the project that the engine is ready to render.
///
/// The splash screen runs first unless the project skips it, and the `ready` event is
/// dispatched only after it is gone, so a project may draw over it from `ready` on.
pub fn announce_ready(core: Arc<Core>) {
    // The project script has run and the first frame has been asked for, so the
    // runtime is ready for inspection. The splash screen is cosmetic and does not
    // hold this back.
    moyu_debugger::mark_ready();

    if !get_engine_config().skip_splash {
        moyu_pal::task::spawn(async move {
            show_splash_screen(core).await;
            dispatch_event(GameEvent::Ready);
        });
    } else {
        dispatch_event(GameEvent::Ready);
    }
}
