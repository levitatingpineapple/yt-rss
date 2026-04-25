use crate::yt::VideoId;
use cached::proc_macro::cached;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{error, info};

pub fn path(cache_dir: &Path, id: &VideoId) -> PathBuf {
    cache_dir.join(format!("{id}.webm"))
}

pub async fn fetch(cache_dir: &Path, id: &VideoId) {
    if path(cache_dir, id).exists() {
        return;
    }
    info!("Caching video: {id}");
    let result = tokio::process::Command::new("yt-dlp")
        .args([
            "-S",
            "res:1080",
            "--embed-chapters",
            "--sponsorblock-remove",
            "sponsor,intro,outro,selfpromo",
            "-o",
            &format!("{}/%(id)s.%(ext)s", cache_dir.display()),
            &format!("https://youtu.be/{id}"),
        ])
        .status()
        .await;
    match result {
        Ok(status) if status.success() => info!("Cached: {id}"),
        Ok(status) => error!("yt-dlp failed for {id}: {status}"),
        Err(e) => error!("yt-dlp spawn failed for {id}: {e}"),
    }
}

pub async fn run_consumer(cache_dir: PathBuf, mut rx: UnboundedReceiver<VideoId>) {
    while let Some(id) = rx.recv().await {
        fetch(&cache_dir, &id).await;
    }
}

// Video sources should be valid for 6 hours
#[cached(time = 20_000, sync_writes = "default")]
pub fn source(id: VideoId) -> String {
    String::from_utf8(
        Command::new("yt-dlp")
            .args([
                "--get-url",
                "--force-ipv4",
                "--no-warnings",
                "-f",
                "18",
                &format!("https://youtu.be/{}", id),
            ])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim_end()
    .to_string()
}
