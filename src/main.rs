mod yt;

use axum::{
    extract::{Path, Request, State},
    http::{header, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use cached::proc_macro::cached;
use clap::{self, Parser};
use std::{process::Command, str::FromStr};
use yt::{fetch_feed, Handle};

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
        match Handle::from_str(&segment) {
            Ok(handle) => match fetch_feed(handle, app_state.host).await {
                Ok(rss_feed) => {
                    if request
                        .headers()
                        .get(header::IF_NONE_MATCH)
                        .map(|h| h.to_str().unwrap_or_default())
                        == Some(&rss_feed.etag)
                    {
                        StatusCode::NOT_MODIFIED.into_response()
                    } else {
                        Response::builder()
                            .header(header::CONTENT_TYPE, "application/json")
                            .header(header::ETAG, rss_feed.etag)
                            .body(rss_feed.channel.to_string().into())
                            .unwrap()
                    }
                }
                Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
            },
            Err(err) => (StatusCode::BAD_REQUEST, err.to_string()).into_response(),
        }
    } else {
        match yt::VideoId::from_str(&segment) {
            Ok(video_id) => Redirect::temporary(&source(video_id)).into_response(),
            Err(err) => (StatusCode::BAD_REQUEST, err.to_string()).into_response(),
        }
    }
}

// Video sources should be valid for 6 hours
#[cached(time = 20_000, sync_writes = "default")]
fn source(id: yt::VideoId) -> String {
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
