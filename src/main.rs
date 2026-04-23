use axum::{
    extract::{Path, Request, State},
    http::{header, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use bytes::Bytes;
use cached::proc_macro::cached;
use clap::*;
use feed_rs::parser;
use std::{collections::hash_map::DefaultHasher, hash::Hasher, process::Command, str::FromStr};
mod feed;
mod id;
use feed::*;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value = "localhost")]
    bind: String,
    #[arg(long, default_value_t = 8080)]
    port: u16,
    #[arg(long, default_value = None)]
    host: Option<String>,
}

impl Args {
    fn host(&self) -> String {
        self.host
            .clone()
            .unwrap_or(format!("http://{}:{}", self.bind, self.port))
    }
}

#[derive(Clone)]
struct AppState {
    host: String,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let data = AppState { host: args.host() };
    let app = Router::new()
        .route("/{segment}", get(dispatch))
        .with_state(data);
    let addr = format!("{}:{}", args.bind, args.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await
}

async fn dispatch(
    State(app_state): State<AppState>,
    Path(segment): Path<String>,
    request: Request,
) -> Response {
    if segment.starts_with("@") {
        let channel_id = match id::Handle::from_str(&segment) {
            Ok(id) => id,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        };
        let channel = channel(channel_id).await;
        let atom_bytes_data = atom_bytes(channel.atom.clone()).await;
        let mut hasher = DefaultHasher::new();
        hasher.write(&atom_bytes_data);
        let hash = format!("{:x}", hasher.finish());
        if let Some(request_etag) = request.headers().get(header::IF_NONE_MATCH) {
            if request_etag.to_str().unwrap_or("") == format!("\"{}\"", hash) {
                return StatusCode::NOT_MODIFIED.into_response();
            }
        }
        Response::builder()
            .header(header::CONTENT_TYPE, "application/json")
            // [ETAG needs quotes](https://datatracker.ietf.org/doc/html/rfc9110#section-8.8.3)
            .header(header::ETAG, format!("\"{}\"", hash))
            .body(
                rss_channel(
                    channel.clone(),
                    parser::parse(atom_bytes_data.as_ref()).expect("YT Atom is valid"),
                    &app_state.host,
                )
                .to_string()
                .into(),
            )
            .unwrap()
    } else {
        if let Ok(video_id) = id::Video::from_str(&segment) {
            Redirect::temporary(&source(video_id)).into_response()
        } else {
            StatusCode::BAD_REQUEST.into_response()
        }
    }
}

#[cached(sync_writes = "default")]
async fn channel(handle: id::Handle) -> Channel {
    Channel::new(
        reqwest::Client::new()
            .get(format!("https://www.youtube.com/{}", handle))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap(),
    )
}

// Atom sources should be valid for 15 minutes
#[cached(time = 900, sync_writes = "default")]
async fn atom_bytes(atom: String) -> Bytes {
    reqwest::get(atom).await.unwrap().bytes().await.unwrap()
}

// Video sources should be valid for 6 hours
#[cached(time = 20_000, sync_writes = "default")]
fn source(id: id::Video) -> String {
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
