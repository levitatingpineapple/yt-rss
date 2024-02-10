use feed_rs::model::Entry;
use regex::Regex;
use serde::Serialize;

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct Channel {
	pub atom: String,
	icon: String
}

impl Channel {
	// Parses the homepage to extract url to the Atom feed and the channel's icon
	pub fn new(homepage: String) -> Channel {
		Channel {
			atom: Regex::new(r#"<link rel="alternate" type="application/rss\+xml" title="RSS" href="(.*?)">"#)
				.expect("Regex is valid")
				.captures(&homepage).unwrap()
				.get(1).unwrap()
				.as_str().to_string(),
			icon: Regex::new(r#"<meta property="og:image" content="(.*?)">"#)
				.expect("Regex is valid")
				.captures(&homepage).unwrap()
				.get(1).unwrap()
				.as_str().to_string()
				.replace("=s900", "=s128") // Request smaller icon
		}
	}
}

#[derive(Serialize)]
pub struct Feed {
	version: String,
	title: String,
	favicon: String,
	items: Vec<Item>
}

impl Feed {
	pub fn new(channel: Channel, atom: feed_rs::model::Feed, host: &str) -> Feed {
		Feed {
			version: "https://jsonfeed.org/version/1.1".to_string(),
			title: atom.title.unwrap().content,
			favicon: channel.icon,
			items: atom.entries
				.into_iter()
				.filter_map(|e| Item::new(e, host))
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
	fn new(entry: Entry, host: &str) -> Option<Item> {
		let id = entry.id.split(":").last()?.to_string();
		let title = entry.title.unwrap().content;
		let description = Item::html_description(&entry.media.first()?.clone().description?.content);
		Some(
			Item {
				id: id.clone(),
				url: entry.links.first().unwrap().clone().href,
				title: title.clone(),
				content_html: description,
				date_published: entry.published.unwrap().to_rfc3339(),
				attachments: vec![
					Attachment { 
						url: format!("{}/{}", host, id),
						mime_type: "video/mp4".to_string(), 
						title: "HD".to_string()
					}
				],
			}
		)
	}
	
	// Makes links clickable and formats line breaks
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
