#![allow(unstable)]
extern crate fragments;
use fragments::Template;
use std::borrow::ToOwned;
use std::fmt;

fn main() {
	//Create a new Template from a string
	let mut template: Template = "Hello, [[:name]]! Is it written as 'white space' or '[[+join white space]]'?".parse().unwrap();

	//Insert something into the `name` placeholder
	template.insert("name".to_owned(), "Peter");

	//Closures and functions with the signature
    //`fn(&[String], &mut fmt::Formatter) -> fmt::Result`
    //will automatically implement the `Generator` trait.
    //This generator will just concatenate the arguments.
    //I expect you to make cooler generators, yourself ;)
	template.insert_generator("join".to_owned(),
        |&: parts: &[String], f: &mut fmt::Formatter| {
            fmt::String::fmt(&parts.concat(), f)
        }
    );

	//Result: "Hello, Peter! Is it written as 'white space' or 'whitespace'?"
	println!("Result: '{}'", template);
}