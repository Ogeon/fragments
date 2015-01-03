extern crate fragments;
use fragments::Template;
use std::fmt::Show;
use std::fmt;

//This function will just concatenate the arguments.
//I expect you to make cooler generators, yourself ;)
fn join(parts: &Vec<String>, f: &mut fmt::Formatter) -> fmt::Result {
	parts.concat().fmt(f)
}

fn main() {
	//Create a new Template from a string
	let mut template: Template = "Hello, [[:name]]! Is it written as 'white space' or '[[+join white space]]'?".parse().unwrap();

	//Insert something into the `name` placeholder
	template.insert("name", "Peter");

	//Functions with the signature `fn(&Vec<String>) -> Box<Show>` will automatically implement the `Generator` trait
	template.insert_generator("join", join as fn(&Vec<String>, &mut fmt::Formatter) -> fmt::Result);

	//Result: "Hello, Peter! Is it written as 'white space' or 'whitespace'?"
	println!("Result: '{}'", template);
}