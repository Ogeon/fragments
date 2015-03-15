fragments
=========

[![Build Status](https://travis-ci.org/Ogeon/fragments.png?branch=master)](https://travis-ci.org/Ogeon/fragments)

A template library for Rust with support for placeholders, conditional content and content generators.

Online documentation can be found [here](http://ogeon.github.io/fragments/doc/fragments/).

#Getting started

##Adding to Your Project
Add the following lines to your `Cargo.toml` file:
```toml
[dependencies.fragments]

git = "https://github.com/Ogeon/fragments"
```

#Examples
The `Template` can both be created from strings and buffers (from a file, for example).
Placeholder tokens (`[[:something]]`) are used to reserve space for dynamic content and
must contain a `:` at the beginning of a label. Multiple placeholders with the same label
will be filled with the same content.

```rust
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
```

```rust
extern crate fragments;
use fragments::Template;
use std::borrow::ToOwned;

fn main() {
	//Create a new Template from a string
	let mut template: Template = "Hello, [[:name]]!".parse().unwrap();

	//Insert something into the `name` placeholder
	template.insert("name".to_owned(), "Peter");

	//Templates can be printed as they are
	//Result: 'Hello, Peter!'
	println!("Result: '{}'", template);
}
```

##Escape Sequences
Any character with a `\` in front of it will be treated as any other character by the parser:
```rust
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
```

##Conditional Content
Parts of the content may be switched on or off with conditional switches.
A conditional part of a template is defined as `[[?something]]...[[/]]`, where the
`[[?...]]` token contains the name of the condition and `[[/]]` marks the end
of the conditional part. The end token may contain anything after the `/`,
which allows them to be labeled, like this: `[[?something]]...[[/something]]`.

```rust
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
```

Conditional parts can also be negated by adding an `!` after the `?`, like this: `[[?!something]]`.

##Generated Content
Content can also be generated, using a generator token: `[[+label arg1 arg2 ...]]`. The label and the arguments are
separated by one or more whitespaces. They can also be quoted to prevent special characters from being parsed:
`[[+"my label" arg1 "[[arg2]]"]]`. The arguments will be passed to an instance of the `Generator` trait and the
result will be inserted into the content.

```rust
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
        |parts: &[String], f: &mut fmt::Formatter| {
            fmt::Display::fmt(&parts.concat(), f)
        }
    );

	//Result: "Hello, Peter! Is it written as 'white space' or 'whitespace'?"
	println!("Result: '{}'", template);
}
```
