extern crate fragments;
use fragments::Template;

fn main() {
	//Create a new Template from a string
	let mut template: Template = from_str("Hello, [[:name]]![[?condition]] The condition is true.[[/condition]]").unwrap();

	//Insert something into the `name` placeholder
	template.insert("name", "Peter");

	//Conditions are false by default, so the second sentence will be disabled
	//Result: 'Hello, Peter!'
	println!("Result: '{}'", template);

	//Let's enable the hidden part of the template
	template.set("condition", true);

	//Result: 'Hello, Peter! The condition is true.'
	println!("Result: '{}'", template);
}