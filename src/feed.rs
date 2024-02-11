use regex::Regex;
use rss::*;

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
				.get(1).expect("Regex contains a capture")
				.as_str().to_string(),
			icon: Regex::new(r#"<meta property="og:image" content="(.*?)">"#)
				.expect("Regex is valid")
				.captures(&homepage).unwrap()
				.get(1).expect("Regex contains a capture")
				.as_str().to_string()
				.replace("=s900", "=s128") // Request smaller icon
		}
	}
}

pub fn rss_feed(channel: Channel, atom: feed_rs::model::Feed, host: &str) -> String {
	ChannelBuilder::default()
		.title(atom.title.unwrap().content)
		.image(
			ImageBuilder::default()
				.url(channel.icon)
				.build()
		)
		.items(
			atom.entries
				.into_iter()
				.filter_map(|e| {
					let id = e.id.split(":").last()?.to_string();
					Some(
						ItemBuilder::default()
							.guid( Guid { value: id.clone(), permalink: false } )
							.link(e.links.first()?.clone().href)
							.title(e.title?.content)
							.content(html_description(&e.media.first()?.clone().description?.content))
							.pub_date(e.published?.to_rfc2822())
							.enclosure(
								EnclosureBuilder::default()
									.url(format!("{}/{}.mp4", host, id))
									.mime_type("video/mp4")
									.build()
							)
							.build()
					)
				})
				.collect::<Vec<rss::Item>>()
		)
		.build()
		.to_string()
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