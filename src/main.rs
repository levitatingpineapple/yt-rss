mod cache;
mod yt;

use axum::{
    extract::{Path, Request, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use cache::source;
use clap::{self, Parser};
use std::path::PathBuf;
use std::str::FromStr;
use tower::ServiceExt;
use tower_http::services::ServeFile;
use tracing_subscriber::EnvFilter;
use yt::{fetch_feed, CacheConfig, Handle, CACHE_CONFIG};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value = "localhost")]
    bind: String,
    #[arg(long, default_value_t = 8080)]
    port: u16,
    #[arg(long, default_value = None)]
    host: Option<String>,
    #[arg(long, default_value = "info")]
    log_level: tracing::Level,
    #[arg(long)]
    cache_dir: Option<PathBuf>,
    #[arg(long, default_value_t = 3)]
    retention_days: i64,
}

impl Args {
    fn host(&self) -> String {
        self.host
            .clone()
            .unwrap_or(format!("http://{}:{}", self.bind, self.port))
    }

    fn cache_dir(&self) -> PathBuf {
        self.cache_dir.clone().unwrap_or(
            PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".cache/yt-rss"),
        )
    }
}

#[derive(Clone)]
struct AppState {
    host: String,
    cache_dir: PathBuf,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(format!("warn,yt_rss={}", args.log_level)))
        .init();
    let cache_dir = args.cache_dir();
    std::fs::create_dir_all(&cache_dir)?;
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    CACHE_CONFIG
        .set(CacheConfig {
            sender: tx,
            retention: chrono::Duration::days(args.retention_days),
        })
        .ok();
    tokio::spawn(cache::run_consumer(cache_dir.clone(), rx));
    let data = AppState {
        host: args.host(),
        cache_dir,
    };
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
) -> Result<Response, AppError> {
    if segment.starts_with("@") {
        let rss_feed = fetch_feed(Handle::from_str(&segment)?, app_state.host).await?;
        if request
            .headers()
            .get(header::IF_NONE_MATCH)
            .map(|h| h.to_str().unwrap_or_default())
            == Some(&rss_feed.etag)
        {
            Ok(StatusCode::NOT_MODIFIED.into_response())
        } else {
            Ok(Response::builder()
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::ETAG, rss_feed.etag)
                .body(rss_feed.body.into())
                .unwrap())
        }
    } else {
        let video_id = yt::VideoId::from_str(&segment)?;
        let path = cache::path(&app_state.cache_dir, &video_id);
        if path.exists() {
            let mut response = ServeFile::new(&path).oneshot(request).await.into_response();
            response
                .headers_mut()
                .insert(header::CONTENT_TYPE, HeaderValue::from_static("video/webm"));
            Ok(response)
        } else {
            Ok(Redirect::temporary(&source(video_id)).into_response())
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("{0}")]
    Id(#[from] yt::IdErr),
    #[error("{0}")]
    Feed(#[from] yt::FeedErr),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Id(id_err) => (StatusCode::BAD_REQUEST, id_err.to_string()).into_response(),
            AppError::Feed(feed_err) => {
                (StatusCode::INTERNAL_SERVER_ERROR, feed_err.to_string()).into_response()
            }
        }
    }
}
