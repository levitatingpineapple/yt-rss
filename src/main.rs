use actix_web::{web::{Path}, HttpServer, App, HttpResponse, get, http};
use ::rss::{ChannelBuilder, Guid, ItemBuilder, Item};

pub mod youtube;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
	HttpServer::new(move || {
		App::new()
			.service(rss)
			.service(source)
	})
	.bind(("localhost", 5550))?
	.run()
	.await
}

#[get("/{handle}")]
async fn rss(handle: Path<String>) -> HttpResponse {
	HttpResponse::Ok()
		.content_type(http::header::ContentType::xml())
		.body(
			ChannelBuilder::default()
				.title(handle.clone())
				.link(format!("https://www.youtube.com/{}", handle.clone()))
				.description("Feed".to_string())
				.items(youtube::ids(handle.clone(), 5)
					.into_iter()
					.map(|id| {
						let info = youtube::info(id.clone());
						let guid = Some(Guid {
							value: id.clone(),
							permalink: false
						});
						ItemBuilder::default()
							.title(info.title)
							.link(format!("youtu.be/{}", id).to_string())
							.guid(guid)
							.description(info.description.clone())
							.content(
								format!(
									"<video controls poster=\"https://i.ytimg.com/vi/{}/maxresdefault.jpg\"><source src=\"http://localhost:5550/source/{}\"></video></hr><p>{}</p>",
									id,
									id,
									info.description
								)
							)
							.build()
					})
					.collect::<Vec<Item>>()
				)
				.build()
				.to_string()
		)
}

#[get("/source/{id}")]
async fn source(id: Path<String>) -> HttpResponse {
	HttpResponse::TemporaryRedirect()
		.append_header((
			"location", youtube::source(id.into_inner())
		)).finish()
}