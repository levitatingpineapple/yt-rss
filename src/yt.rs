use cached::proc_macro::cached;
use feed_rs::parser::parse;
use once_cell::sync::Lazy;
use regex::Regex;
use rss::{ChannelBuilder, EnclosureBuilder, Guid, ImageBuilder, ItemBuilder};
use scraper::{Html, Selector};
use std::{fmt, str::FromStr};

// MARK: Handle

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Handle(String);

impl FromStr for Handle {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.strip_prefix('@')
            .filter(|name| (3..=30).contains(&name.len()) && chars_allowed(name))
            .map(|name| Self(name.to_string()))
            .ok_or(Error::InvalidChannelId)
    }
}

impl fmt::Display for Handle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{}", self.0)
    }
}

static RSS_SEL: Lazy<Selector> = Lazy::new(|| {
    Selector::parse(r#"link[rel="alternate"][type="application/rss+xml"]"#).expect("valid selector")
});

static ICON_SEL: Lazy<Selector> =
    Lazy::new(|| Selector::parse(r#"meta[property="og:image"]"#).expect("valid selector"));

#[derive(Debug, thiserror::Error, Clone)]
pub enum HandleErr {
    #[error("reqwest error: {0}")]
    Reqwest(String),
    #[error("Feed link not found")]
    FeedLinkNotFound,
    #[error("Icon not found")]
    IconNotFound,
    #[error("Feed parse error")]
    ParseFeed(String),
}

#[derive(Debug, Clone)]
struct ChannelURLs {
    feed: String,
    icon: String,
}

#[cached(sync_writes = "default")]
async fn fetch_urls(handle: Handle) -> Result<ChannelURLs, HandleErr> {
    let text = reqwest::Client::new()
        .get(format!("https://www.youtube.com/{handle}"))
        .send()
        .await
        .map_err(|e| HandleErr::Reqwest(e.to_string()))?
        .text()
        .await
        .map_err(|e| HandleErr::Reqwest(e.to_string()))?;
    let document = Html::parse_document(&text);
    Ok(ChannelURLs {
        feed: document
            .select(&RSS_SEL)
            .next()
            .and_then(|el| el.value().attr("href"))
            .ok_or(HandleErr::FeedLinkNotFound)?
            .to_string(),
        icon: document
            .select(&ICON_SEL)
            .next()
            .and_then(|el| el.value().attr("content"))
            .ok_or(HandleErr::IconNotFound)?
            .replace("=s900", "=s128"),
    })
}

#[derive(Clone)]
pub struct RssFeed {
    pub channel: rss::Channel,
    pub etag: String,
}

#[cached(time = 900, sync_writes = "default")]
pub async fn fetch_feed(handle: Handle, host: String) -> Result<RssFeed, HandleErr> {
    let urls = fetch_urls(handle.clone()).await?;
    let bytes = reqwest::Client::new()
        .get(format!("https://www.youtube.com/{handle}"))
        .send()
        .await
        .map_err(|e| HandleErr::Reqwest(e.to_string()))?
        .bytes()
        .await
        .map_err(|e| HandleErr::Reqwest(e.to_string()))?;
    let feed = parse(bytes.as_ref()).map_err(|err| HandleErr::ParseFeed(err.to_string()))?;
    Ok(RssFeed {
        channel: ChannelBuilder::default()
            .title(feed.title.unwrap().content)
            .image(ImageBuilder::default().url(urls.icon).build())
            .items(
                feed.entries
                    .into_iter()
                    .filter_map(|e| {
                        let id = e.id.split(":").last()?.to_string();
                        Some(
                            ItemBuilder::default()
                                .guid(Guid {
                                    value: id.clone(),
                                    permalink: false,
                                })
                                .link(e.links.first()?.clone().href)
                                .title(e.title?.content)
                                .content(html_description(
                                    &e.media.first()?.clone().description?.content,
                                ))
                                .pub_date(e.published?.to_rfc2822())
                                .enclosure(
                                    EnclosureBuilder::default()
                                        .url(format!("{}/{}", host, id))
                                        .mime_type("video/mp4")
                                        .build(),
                                )
                                .build(),
                        )
                    })
                    .collect::<Vec<rss::Item>>(),
            )
            .build(),
        etag: format!("\"{:x}\"", md5::compute(bytes)),
    })
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

// MARK: VideoId

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VideoId(String);

impl FromStr for VideoId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() == 11 && chars_allowed(s) {
            Ok(Self(s.to_string()))
        } else {
            Err(Error::InvalidVideoId)
        }
    }
}

impl fmt::Display for VideoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn chars_allowed(s: &str) -> bool {
    s.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid channel ID")]
    InvalidChannelId,
    #[error("Invalid video ID")]
    InvalidVideoId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle() {
        assert!("@abc".parse::<Handle>().is_ok());
        assert!("@123456789012345678901234567890".parse::<Handle>().is_ok());
        assert!("@ab".parse::<Handle>().is_err());
        assert!("@1234567890123456789012345678901"
            .parse::<Handle>()
            .is_err());
        assert!("@abc!".parse::<Handle>().is_err());
        assert!("@abc def".parse::<Handle>().is_err());
        assert!("@abc/def".parse::<Handle>().is_err());
        assert!("abc".parse::<Handle>().is_err());
    }

    #[test]
    fn test_video() {
        assert!("abcdefghijk".parse::<VideoId>().is_ok());
        assert!("abcDEF123-_".parse::<VideoId>().is_ok());
        assert!("abcdefghij".parse::<VideoId>().is_err());
        assert!("abcdefghijkl".parse::<VideoId>().is_err());
        assert!("abcdefghij!".parse::<VideoId>().is_err());
        assert!("abcde fghij".parse::<VideoId>().is_err());
        assert!("abcdefghij/".parse::<VideoId>().is_err());
    }

    #[tokio::test]
    async fn test_fetch() {
        let handle = Handle::from_str("@Fireship").unwrap();
        let info = fetch_urls(handle).await;
        dbg!(&info);
        assert!(info.is_ok());
    }
}
