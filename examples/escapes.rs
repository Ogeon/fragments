#![feature(collections, core)]
extern crate fragments;
use fragments::Template;
use std::borrow::ToOwned;

fn main() {
	//Create a new Template from a string
	//We will have to escape the escapes when writing it as a string literal,
	//but it's the same as '[...]\[[:this]] and escape them like \\\[[:this]][...]'
	let mut template: Template = "Hello, [[:name]]! Write placeholders like \\[[:this]] and escape them like \\\\\\[[:this]]".parse().unwrap();

	//Insert something into the `name` placeholder
	template.insert("name".to_owned(), "Peter");

	//Templates can be printed as they are
	//Result: 'Hello, Peter! Write placeholders like [[:this]] and escape them like \[[:this]]'
	println!("Result: '{}'", template);
}