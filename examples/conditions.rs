extern crate fragments;
use fragments::Template;
use std::borrow::ToOwned;

fn main() {
	//Create a new Template from a string
	let mut template: Template = "Hello, [[:name]]![[?condition]] The condition is true.[[/condition]]".parse().unwrap();

	//Insert something into the `name` placeholder
	template.insert("name".to_owned(), "Peter");

	//Conditions are false by default, so the second sentence will be disabled
	//Result: 'Hello, Peter!'
	println!("Result: '{}'", template);

	//Let's enable the hidden part of the template
	template.set("condition".to_owned(), true);

	//Result: 'Hello, Peter! The condition is true.'
	println!("Result: '{}'", template);
}