use actix_web::{http::header::*, web::*, *};
use cached::proc_macro::cached;
use clap::*;
use feed_rs::parser;
use std::{
	collections::hash_map::DefaultHasher, 
	hash::Hasher, 
	process::Command
};
mod feed;
use feed::*;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
	/// Bind address
	#[arg(long, default_value = "localhost")]
	bind: String,

	/// Port to listen on
	#[arg(long, default_value_t = 8080)]
	port: u16,
	
	/// Optional Public URL, if service is hosted behind a reverse proxy
	#[arg(long, default_value = None)]
	host: Option<String>
}

impl Args {
	fn host(&self) -> String {
		self.host.clone()
			.unwrap_or(format!("http://{}:{}", self.bind, self.port))
	}
}

#[derive(Clone)]
struct AppState { host: String }

#[actix_web::main]
async fn main() -> std::io::Result<()> {
	let args = Args::parse();
	let data = AppState { host: args.host() };
	HttpServer::new(move || {
		App::new()
			.app_data(Data::new(data.clone()))
			.service(rss)
			.service(src)
	})
	.bind((args.bind, args.port))?
	.run()
	.await
}

// Returns RSS feed for a channel
// Regex: youtube channel's unique handle
#[get("/{handle:@[A-Za-z0-9-_.]{3,30}}")]
async fn rss(handle: Path<String>, request: HttpRequest, app_state: Data<AppState>) -> HttpResponse {
	// Fetch sources
	let channel = channel(handle.into_inner()).await;
	let atom_bytes = atom_bytes(channel.atom.clone()).await;
	
	// Calculate hash
	let mut hasher = DefaultHasher::new();
	hasher.write(&atom_bytes);
	let hash = format!("{:x}", hasher.finish());
	
	// Check if ETag matches and return 304 (not modified) if it does
	if let Some(request_etag) = request.headers().get(http::header::IF_NONE_MATCH) {
		if request_etag.to_str().expect("ETag is ASCII Encoded") == format!("{}", EntityTag::new_strong(hash.clone())) {
			return HttpResponse::NotModified().finish()
		}
	}
	
	// If not create and return feed
	HttpResponse::Ok()
		.content_type(ContentType::json())
		.insert_header(ETag(EntityTag::new_strong(hash)))
		.body(
			rss_feed(
				channel.clone(), 
				parser::parse(atom_bytes.as_ref()).expect("YT Atom is valid"),
				&app_state.host
			)
		)
}

// Returns video source
// Regex: youtube video's unique id
#[get("/{id:[A-Za-z0-9-_.]{11}}.mp4")]
async fn src(id: Path<String>) -> HttpResponse {
	HttpResponse::TemporaryRedirect()
		.append_header((
			"location", 
			source(id.into_inner())
		))
		.finish()
}

// Resolved handles are cached in memory
#[cached(sync_writes = true)]
async fn channel(handle: String) -> Channel {
	Channel::new(
		reqwest::Client::new()
			.get(format!("https://www.youtube.com/{handle}"))
			.send().await.unwrap()
			.text().await.unwrap()
	)
}

// Atom sources should be valid for 15 minutes
#[cached(time=900, sync_writes = true)]
async fn atom_bytes(atom: String) -> actix_web::web::Bytes {
	reqwest::get(atom).await.unwrap()
		.bytes().await.unwrap()
}

// Video sources should be valid for 6 hours
#[cached(time=20_000, sync_writes = true)]
fn source(id: String) -> String {
	String::from_utf8(
		Command::new("yt-dlp")
		.args([
			"--get-url",
			"--force-ipv4",
			"--no-warnings",
			"-f", "22",
			&format!("https://youtu.be/{id}")
		]).output().unwrap().stdout
	).unwrap()
	.trim_end()
	.to_string()
}