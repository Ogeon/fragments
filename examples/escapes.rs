extern crate fragments;
use fragments::Template;

fn main() {
	//Create a new Template from a string
	//We will have to escape the escapes when writing it as a string literal,
	//but it's the same as '[...]\[[:this]] and escape them like \\\[[:this]][...]'
	let mut template: Template = from_str("Hello, [[:name]]! Write placeholders like \\[[:this]] and escape them like \\\\\\[[:this]]").unwrap();

	//Insert something into the `name` placeholder
	template.insert("name", "Peter");

	//Templates can be printed as they are
	//Result: 'Hello, Peter! Write placeholders like [[:this]] and escape them like \[[:this]]'
	println!("Result: '{}'", template);
}