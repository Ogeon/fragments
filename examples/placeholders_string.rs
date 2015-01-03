extern crate fragments;
use fragments::Template;

fn main() {
	//Create a new Template from a string
	let mut template: Template = "Hello, [[:name]]!".parse().unwrap();

	//Insert something into the `name` placeholder
	template.insert("name", "Peter");

	//Templates can be printed as they are
	//Result: 'Hello, Peter!'
	println!("Result: '{}'", template);
}