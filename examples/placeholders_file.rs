#![feature(old_path, old_io)]
extern crate fragments;
use fragments::Template;
use std::borrow::ToOwned;
use std::old_io::{BufferedReader, File};
use std::old_path::Path;

fn main() {
	//Load the content of a file into a Template
	//The file contains the text 'Hello, [[:name]]!', in this example
	let file = File::open(&Path::new("path/to/my/template.txt"));
	let mut template = match Template::from_buffer(&mut BufferedReader::new(file)) {
		Ok(template) => template,
		Err(e) => panic!(e)
	};

	//Insert something into the `name` placeholder
	template.insert("name".to_owned(), "Peter");

	//Templates can be printed as they are
	//Result: 'Hello, Peter!'
	println!("Result: '{}'", template);
}