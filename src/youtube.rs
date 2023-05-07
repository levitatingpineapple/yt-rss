use std::{process::Command, io::{BufRead}};

#[derive(Debug, Clone)]
pub struct Info {
	pub id: String,
	pub date: String,
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
	.output().unwrap()
	.stdout.lines()
	.collect::<Result<Vec<String>, std::io::Error>>().unwrap()
}

pub fn info(id: String) -> Info {
	let command = Command::new("yt-dlp")
			.args([
				"--print", "%(title)s\n%(upload_date)s\n%(description)s",
				format!("https://youtu.be/{}", id.as_str()).as_str()
			])
			.output().unwrap();
	let output = String::from_utf8(command.stdout).unwrap();
	let (title, rest) = output
		.split_once("\n")
		.unwrap();
	let (date, description) = rest
		.split_once("\n")
		.unwrap();
	Info {
		id: id,
		date: date.to_string(),
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
		.output().unwrap()
		.stdout
		.lines().next().unwrap().unwrap()
}
