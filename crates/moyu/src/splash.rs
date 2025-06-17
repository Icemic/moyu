use std::sync::Arc;

pub async fn show_splash_screen(core: Arc<moyu_core::core::Core>) {
    use moyu_core::resource::TextureId;
    use moyu_core::traits::NodeBaseTrait;
    use moyu_nodes::nodes::*;
    use moyu_pal::sync::RwLock;

    let n = Sprite::new("splash".to_string());
    let node = Arc::new(RwLock::new(n));

    // create sprite and load texture
    {
        let mut node = node.write();
        let texture_id = Arc::new(TextureId::Data(
            include_bytes!("../static/logo.png").to_vec(),
        ));
        node.next_texture_id.store(Some(texture_id));

        // logo size is 3840x2400 (16:10), scale as cover the whole screen
        let (width, height) = core.stage_size().logical_size_f32();
        let scale_x = width as f32 / 3840.0;
        let scale_y = height as f32 / 2400.0;
        let scale = scale_x.max(scale_y);
        node.base_mut().set_scale(scale, scale);
        node.base_mut().set_translate(
            (width as f32 / 2.0) - (3840.0 * scale / 2.0),
            (height as f32 / 2.0) - (2400.0 * scale / 2.0),
        );
        node.base_mut().set_opacity(0.0);
        node.base_mut().set_interactive(false);
    }

    // add to root node
    {
        let mut root = core.root_node().write();
        root.base_mut().add_child(node.clone());
    }

    let instant = moyu_pal::time::Instant::now();

    // fade-in-out effect
    loop {
        let elapsed = instant.elapsed();
        if elapsed.as_millis() > 2000 {
            break;
        }

        let opacity = fade_in_out(elapsed.as_millis() as f32 / 2000.0);

        if let Some(mut node) = node.try_write() {
            node.base_mut().set_opacity(opacity);
        }

        // wait for next frame
        moyu_pal::time::sleep(std::time::Duration::from_millis(7)).await;
    }

    // remove splash screen
    {
        let mut root = core.root_node().write();
        root.base_mut().remove_child(node);
    }
}

#[inline]
fn fade_in_out(progress: f32) -> f32 {
    // progress is in range [0.0, 1.0]
    // fade in for first half, fade out for second half
    if progress < 0.4 {
        ease_in_quart(progress * 2.5) // fade in
    } else if progress < 0.6 {
        1.0 // hold
    } else {
        1.0 - ease_out_cubic((progress - 0.6) * 2.5) // fade out
    }
}

#[inline]
fn ease_in_quart(x: f32) -> f32 {
    x * x * x * x
}

#[inline]
fn ease_out_cubic(x: f32) -> f32 {
    1.0 - (1.0 - x).powi(3)
}
