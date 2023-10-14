use std::{process::Command, io::BufRead};
use actix_web::{web::Path, HttpResponse, get, http, App, HttpServer};
use feed_rs::{model::Entry, parser};
use regex::Regex;
use ::rss::{ChannelBuilder, Guid, ItemBuilder, Item};
use cached::proc_macro::cached;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
	HttpServer::new(move || {
		App::new()
			.service(rss)
			.service(src)
	})
	.bind(("localhost", 7777))?
	.run()
	.await
}

// Returns RSS feed for a channel
// Regex: youtube channel's unique handle
#[get("/{handle:@[A-Za-z0-9-_.]{3,30}}")]
async fn rss(handle: Path<String>) -> HttpResponse {
	let channel_id = channel_id(handle.into_inner());
	let body = reqwest::get(format!("https://www.youtube.com/feeds/videos.xml?channel_id={channel_id}"))
		.await.unwrap()
		.text().await.unwrap();
	let feed = parser::parse(body.as_bytes()).unwrap();
	HttpResponse::Ok()
		.content_type(http::header::ContentType::xml())
		.body(
			ChannelBuilder::default()
				.title(feed.title.unwrap().content)
				.link(feed.links.last().unwrap().clone().href)
				.items(
					feed.entries
						.into_iter()
						.filter_map(|e| item(e))
						.collect::<Vec<Item>>()
				)
				.build()
				.to_string()
	)
}

// Dynamically redirects to source of the video.
// Valid for few hours. TODO: Add time based caching.
// Regex: Id of the youtube video
#[get("/{id:[A-Za-z0-9-_.]{11}}")]
async fn src(id: Path<String>) -> HttpResponse {
	let id = id.into_inner();
	let location = Command::new("yt-dlp")
		.args([
			"--get-url",
			"--force-ipv4",
			"--no-warnings",
			"-f", "22",
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
	let content: String = format!(
r#"<video controls width="1280" heigh="720" poster="https://img.youtube.com/vi/{id}/maxresdefault.jpg">
	<source src="http://localhost:7777/{id}">
</video>
<p>{description}</p>"#,
	);
	Some(
		ItemBuilder::default()
			.guid(Guid { value: id, permalink: false })
			.title(entry.title?.content)
			.link(entry.links.first()?.clone().href)
			.pub_date(entry.published?.to_rfc2822())
			.content(content)
			.build()
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