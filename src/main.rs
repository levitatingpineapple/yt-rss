use std::{process::Command, io::BufRead};
use actix_web::{web::Path, HttpResponse, get, http, App, HttpServer};
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
	.bind(("localhost", 8080))?
	.run()
	.await
}

#[derive(Serialize)]
struct Feed {
	version: String,
	title: String,
	home_page_url: String,
	feed_url: String,
	// favicon: String,
	items: Vec<Item>
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

#[derive(Serialize)]
struct Attachment {
	url: String,
	mime_type: String,
	title: String
}

// Returns RSS feed for a channel
// Regex: youtube channel's unique handle
#[get("/{handle:@[A-Za-z0-9-_.]{3,30}}")]
async fn rss(handle: Path<String>) -> HttpResponse {
	let channel_id = channel_id(handle.into_inner());
	let body = reqwest::get(format!("https://www.youtube.com/feeds/videos.xml?channel_id={channel_id}"))
		.await.unwrap()
		.text().await.unwrap();
	let atom = parser::parse(body.as_bytes()).unwrap();
	let output_feed = Feed {
		version: "https://jsonfeed.org/version/1.1".to_string(),
		title: atom.title.unwrap().content,
		home_page_url: "https://www.youtube.com".to_string(),
		feed_url: atom.links.last().unwrap().clone().href,
		items: atom.entries
			.into_iter()
			.filter_map(|e| item(e))
			.collect::<Vec<Item>>()
	};
	

	HttpResponse::Ok()
		.content_type(http::header::ContentType::xml())
		.body(serde_json::to_string(&output_feed).unwrap())
}

#[get("/{id:[A-Za-z0-9-_.]{11}}/{quality:(sd|hd)}")]
async fn src(path: Path<(String, String)>) -> HttpResponse {
	let inner = path.into_inner();
	let id = inner.0;
	let location = Command::new("yt-dlp")
		.args([
			"--get-url",
			"--force-ipv4",
			"--no-warnings",
			"-f", (if inner.1.as_str() == "sd" {"18"} else {"22"}),
			&format!("https://youtu.be/{id}")
		])
		.output().unwrap()
		.stdout
		.lines().next().unwrap().unwrap();
	HttpResponse::TemporaryRedirect()
		.append_header(("location", location))
		.finish()
}

#[cached]
fn channel_id(handle: String) -> String {
	let output = String::from_utf8(
		Command::new("yt-dlp")
			.args([
				"--print", "channel_url",
				"--playlist-items", "1",
				&format!("https://www.youtube.com/{handle}")
			])
			.output().unwrap()
			.stdout
	).unwrap();
	Regex::new(r#"/([A-Za-z0-9-_]{24})\n"#).expect("Regex is valid")
		.captures(&output).unwrap()
		.get(1).unwrap()
		.as_str().to_string()
}

// Maps youtube's feed entry to RSS item with html content
fn item(entry: Entry) -> Option<Item> {
	let id = entry.id.split(":").last()?.to_string();
	let description = html_description(
		&entry.media.first()?.clone().description?.content
	);
	Some(
		Item {
			id: id.clone(),
			url: entry.links.first().unwrap().clone().href,
			title: entry.title.unwrap().content,
			content_html: description,
			date_published: entry.published.unwrap().to_rfc3339(),
			attachments: vec![
				Attachment { 
					url: format!("http://localhost:8080/{id}/sd"), 
					mime_type: "video/mp4".to_string(), 
					title: "360p".to_string()
				},
				Attachment { 
					url: format!("http://localhost:8080/{id}/hd"), 
					mime_type: "video/mp4".to_string(), 
					title: "720p".to_string()
				}
			],
		}
	)
}

// Formats links and newlines as html
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