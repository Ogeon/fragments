extern crate fragments;
use fragments::Template;
use std::borrow::ToOwned;
use std::io::BufReader;
use std::fs::File;
use std::path::Path;

fn main() {
	//Load the content of a file into a Template
	//The file contains the text 'Hello, [[:name]]!', in this example
	let file = File::open(&Path::new("path/to/my/template.txt")).unwrap();
	let mut template = match Template::from_buffer(&mut BufReader::new(file)) {
		Ok(template) => template,
		Err(e) => panic!(e)
	};

	//Insert something into the `name` placeholder
	template.insert("name".to_owned(), "Peter");

	//Templates can be printed as they are
	//Result: 'Hello, Peter!'
	println!("Result: '{}'", template);
}