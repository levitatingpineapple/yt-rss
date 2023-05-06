use std::{process::Command, io::{BufRead}};
use cached::proc_macro::cached;

#[derive(Debug, Clone)]
pub struct Info {
	pub title: String,
	pub description: String
}

pub fn ids(handle: String, max: u32) -> Vec<String> {
Command::new("yt-dlp")
	.args([
		"--flat-playlist",
		"--print", "%(id)s",
		"--playlist-end", &(max).to_string(),
		format!("https://www.youtube.com/{}" ,handle).as_str()
	])
	.output()
	.unwrap()
	.stdout.lines()
	.collect::<Result<Vec<String>, std::io::Error>>()
	.unwrap()
}

#[cached]
pub fn info(id: String) -> Info {
	let stdout_string = String::from_utf8(
		Command::new("yt-dlp")
			.args([
				"--print", "%(title)s\n%(description)s",
				id.as_str()
			])
			.output()
			.unwrap()
			.stdout
	).unwrap();
	let (title, description) = stdout_string
		.split_once("\n")
		.unwrap();
	Info {
		title: title.to_string(),
		description: description.to_string()
	}
}

pub fn source(id: String) -> String {
	Command::new("yt-dlp")
		.args([
			"--get-url",
			"-f", "22,18",
			id.as_str()
		])
		.output()
		.unwrap()
		.stdout
		.lines().next().unwrap().unwrap()
}
