pub use std::time::*;
pub use tokio::time::sleep;

pub async fn wait_animation_frame() {
    sleep(std::time::Duration::from_millis(4)).await;
}
