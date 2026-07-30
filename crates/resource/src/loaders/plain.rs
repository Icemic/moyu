use std::sync::Arc;

use moyu_pal::url::Url;
use moyu_pal::{fs, task};

use crate::types::{Plain, PlainStatus};

pub(crate) fn load_plain(url: &Url) -> Arc<Plain> {
    log::debug!("loading plain resource from {}", url);

    let plain = Arc::new(Plain::new());

    {
        let plain = plain.clone();
        let url = url.to_owned();
        task::spawn(async move {
            let bytes = match fs::read(&url).await {
                Ok(bytes) => bytes,
                Err(err) => {
                    log::error!("failed to read plain resource '{}': {}", url, err);
                    plain.set_status(PlainStatus::Error);
                    return;
                }
            };

            let content = match String::from_utf8(bytes) {
                Ok(content) => content,
                Err(err) => {
                    log::error!("plain resource '{}' is not valid UTF-8: {}", url, err);
                    plain.set_status(PlainStatus::Error);
                    return;
                }
            };

            plain.set_content(content);
            plain.set_status(PlainStatus::Ready);
            log::debug!("plain resource '{}' loaded", url);
        });
    }

    plain
}
