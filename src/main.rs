use actix_web::{web::{Path}, HttpServer, App, HttpResponse, get, http};
use chrono::NaiveDate;
use ::rss::{ChannelBuilder, Guid, ItemBuilder, Item};
use dpc_pariter::IteratorExt as _;
use cached::proc_macro::cached;

pub mod youtube;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
	HttpServer::new(move || {
		App::new()
			.service(rss)
			.service(source)
	})
	.bind(("localhost", 7777))?
	.run()
	.await
}

#[get("/yt-rss/{handle}")]
async fn rss(handle: Path<String>) -> HttpResponse {
	HttpResponse::Ok()
		.content_type(http::header::ContentType::xml())
		.body(
			ChannelBuilder::default()
				.title(handle.clone())
				.link(format!("https://www.youtube.com/{}", handle.clone()))
				.description("Feed".to_string())
				.items(youtube::ids(handle.clone(), 30)
					.into_iter()
					.parallel_map(item)
					.collect::<Vec<Item>>()
				)
				.build()
				.to_string()
		)
}

#[get("/yt-rss/source/{id}")]
async fn source(id: Path<String>) -> HttpResponse {
	HttpResponse::TemporaryRedirect()
		.append_header((
			"location", youtube::source(id.into_inner())
		)).finish()
}

#[cached]
fn item(id: String) -> Item { 
	let info = youtube::info(id);
	let content: String = format!(
r#"<video controls poster="https://i.ytimg.com/vi/{}/maxresdefault.jpg">
	<source src="https://levitatingpineapple.com/yt-rss/source/{}">
</video>
<p>{}</p>"#,
		info.id,
		info.id,
		info.description
	);
	let pub_date = NaiveDate::parse_from_str(&info.date, "%Y%m%d").ok()
		.and_then(|date| 
			Some(date.format("%a, %d %b %Y 00:00:00 UTC").to_string())
		);
	ItemBuilder::default()
		.title(Some(info.title))
		.link(Some(format!("https://youtu.be/{}", info.id).to_string()))
		.guid(Some(Guid { value: info.id, permalink: false }))
		.pub_date(pub_date)
		.description(Some(info.description))
		.content(Some(content))
		.build()
}