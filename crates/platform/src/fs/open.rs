use std::io::Cursor;

use anyhow::Result;
use url::Url;

use super::read;

/// Open a file from a URL, returns a `Cursor<Vec<u8>>` which is [`Read`](std::io::Read) + [`Seek`](std::io::Seek)
pub async fn open(url: &Url) -> Result<Cursor<Vec<u8>>> {
    Ok(Cursor::new(read(url).await?))
}
