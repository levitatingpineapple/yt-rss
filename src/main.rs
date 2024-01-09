use std::{process::Command, io::BufRead, collections::hash_map::DefaultHasher, hash::Hasher};
use actix_web::{web::Path, HttpResponse, HttpRequest, get, http::{self, header::EntityTag}, App, HttpServer};
use feed_rs::{model::Entry, parser};
use regex::Regex;
use cached::proc_macro::cached;
use serde::Serialize;
use serde_json;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
	HttpServer::new(move || {
		App::new()
			.service(rss)
			.service(src)
	})
	.bind(("127.0.0.1", 8080))?
	.run()
	.await
}

// Returns RSS feed for a channel
// Regex: youtube channel's unique handle
#[get("/{handle:@[A-Za-z0-9-_.]{3,30}}")]
async fn rss(handle: Path<String>, request: HttpRequest) -> HttpResponse {
	print!("Request: {:?}", request);
	
	// Fetch sources
	let channel = channel(handle.into_inner()).await;
	let atom_bytes = atom_bytes(channel.atom.clone()).await;
	
	// Calculate hash
	let mut hasher = DefaultHasher::new();
	hasher.write(&atom_bytes);
	let hash = format!("{:x}", hasher.finish());
	
	// Check if ETag matches and return 304 if it does
	if let Some(request_etag) = request.headers().get(http::header::IF_NONE_MATCH) {
		if request_etag.to_str().unwrap() == format!("{}", EntityTag::new_strong(hash.clone())) {
			println!("ETAG MATCH");
			return HttpResponse::NotModified().finish()
		}
	}
	
	// If not create and return feed
	HttpResponse::Ok()
		.content_type(http::header::ContentType::json())
		.insert_header(http::header::ETag(EntityTag::new_strong(hash)))
		.body(json_feed(channel, &atom_bytes).await)
}

#[get("/{quality:(sd|hd)}/{id:[A-Za-z0-9-_.]{11}}")]
async fn src(path: Path<(String, String)>) -> HttpResponse {
	let inner = path.into_inner();
	let sources = sources(inner.1);
	HttpResponse::TemporaryRedirect()
		.append_header((
			"location", 
			if inner.0 == "sd".to_string() { 
				sources.first() 
			} else { 
				sources.last() 
			}.unwrap().to_string()
		))
		.finish()
}

#[cached(sync_writes = true)]
async fn channel(handle: String) -> Channel {
	println!("Channel: {}", handle);
	let request = reqwest::Client::new()
		.get(format!("https://www.youtube.com/{handle}"))
		.send().await.unwrap()
		.text().await.unwrap();
	Channel {
		atom: Regex::new(r#"<link rel="alternate" type="application/rss\+xml" title="RSS" href="(.*?)">"#)
			.expect("Regex is valid")
			.captures(&request).unwrap()
			.get(1).unwrap()
			.as_str().to_string(),
		icon: Regex::new(r#"<meta property="og:image" content="(.*?)">"#)
			.expect("Regex is valid")
			.captures(&request).unwrap()
			.get(1).unwrap()
			.as_str().to_string().replace("=s900", "=s64")
	}
}

#[cached(time=900, sync_writes = true)]
async fn atom_bytes(atom: String) -> actix_web::web::Bytes {
	reqwest::get(atom).await.unwrap()
		.bytes().await.unwrap()
}

async fn json_feed(channel: Channel, atom_bytes: &[u8]) -> Vec<u8> {
	serde_json::to_vec(
		&Feed::new(
			channel.clone(), 
			parser::parse(atom_bytes).unwrap()
		)
	).unwrap()
}

// Video sources should be valid for 6 hours
#[cached(time=20_000, sync_writes = true)]
fn sources(id: String) -> Vec<String> {
	println!("Source: {}", id);
	Command::new("yt-dlp")
		.args([
			"--get-url",
			"--force-ipv4",
			"--no-warnings",
			"-f", "18,22",
			&format!("https://youtu.be/{id}")
		])
		.output().unwrap()
		.stdout.lines()
		.into_iter()
		.filter_map(|l| l.ok())
		.into_iter()
		.collect::<Vec<String>>()
}


#[derive(Clone, Hash, PartialEq, Eq)]
struct Channel {
	atom: String,
	icon: String
}

#[derive(Serialize)]
struct Feed {
	version: String,
	title: String,
	favicon: String,
	items: Vec<Item>
}

impl Feed {
	fn new(channel: Channel, atom: feed_rs::model::Feed) -> Feed {
		Feed {
			version: "https://jsonfeed.org/version/1.1".to_string(),
			title: atom.title.unwrap().content,
			favicon: channel.icon,
			items: atom.entries
				.into_iter()
				.filter_map(|e| Item::new(e))
				.collect::<Vec<Item>>()
		}
	}
}

#[derive(Serialize)]
struct Item {
	id: String,
	url: String,
	title: String,
	content_html: String,
	date_published: String,
	attachments: Vec<Attachment>
}

impl Item {
	fn new(entry: Entry) -> Option<Item> {
		let id = entry.id.split(":").last()?.to_string();
		let title = entry.title.unwrap().content;
		let description = Self::html_description(&entry.media.first()?.clone().description?.content);
		Some(
			Item {
				id: id.clone(),
				url: entry.links.first().unwrap().clone().href,
				title: title.clone(),
				content_html: description,
				date_published: entry.published.unwrap().to_rfc3339(),
				attachments: vec![
					Attachment { 
						url: format!("http://home:8080/sd/{id}"), 
						mime_type: "video/mp4".to_string(), 
						title: "SD".to_string()
					},
					Attachment { 
						url: format!("http://home:8080/hd/{id}"), 
						mime_type: "video/mp4".to_string(), 
						title: "HD".to_string()
					}
				],
			}
		)
	}
	
	fn html_description(string: &str) -> String {
		let link_or_line_break = Regex::new(r#"(https?://[^\s]+|\n)"#).expect("Regex is valid");
		let mut match_end: usize = 0;
		let mut html = String::new();
		link_or_line_break
			.captures_iter(string)
			.map(|c| c.get(0).unwrap())
			.for_each(|regex_match| {
				let match_string = regex_match.as_str();
				let replacement = if match_string == "\n" {
					"<br>".to_string()
				} else {
					format!(r#"<a href="{match_string}">{match_string}</a>"#)
				};
				html.push_str(&string[match_end..regex_match.start()]);
				html.push_str(&replacement);
				match_end = regex_match.end();
			});
		html
	}
}

#[derive(Serialize)]
struct Attachment {
	url: String,
	mime_type: String,
	title: String
}
