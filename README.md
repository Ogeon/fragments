fragments
=========

[![Build Status](https://travis-ci.org/Ogeon/fragments.png?branch=master)](https://travis-ci.org/Ogeon/fragments)

A simple template library for Rust with support for placeholders and conditional content.

Online documentation can be found [here](http://www.rust-ci.org/Ogeon/fragments/doc/fragments/).

#Getting started

Simply clone the repository an build it like this:

```shell
git clone https://github.com/Ogeon/fragments.git
cd fragments
make
```

The library files will be placed in a new directory called `lib/`. You can also run `make docs` to generate documentation.

#Examples
The `Template` can both be created from strings and buffers (from a file, for example).
Placeholder tokens (`[[:something]]`) are used to reserve space for dynamic content and
must contain a `:` at the beginning of a label. Multiple placeholders with the same label
will be filled with the same content.

```rust
extern crate fragments;
use fragments::Template;
use std::io::{BufferedReader, File};
use std::path::Path;

fn main() {
	//Load the content of a file into a Template
	//The file contains the text 'Hello, [[:name]]!', in this example
	let file = File::open(&Path::new("path/to/my/template.txt"));
	let mut template = Template::from_buffer(&mut BufferedReader::new(file));

	//Insert something into the `name` placeholder
	//The ~(...) pattern is currently necessary because of how the compiler handles ~str
	template.insert("name", ~("Peter"));

	//Templates can be printed as they are
	//Result: 'Hello, Peter!'
	println!("Result: '{}'", template);
}
```

```rust
extern crate fragments;
use fragments::Template;

fn main() {
	//Create a new Template from a string
	let mut template: Template = from_str("Hello, [[:name]]!").unwrap();

	//Insert something into the `name` placeholder
	//The ~(...) pattern is currently necessary because of how the compiler handles ~str
	template.insert("name", ~("Peter"));

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

fn main() {
	//Create a new Template from a string
	//We will have to escape the escapes when writing it as a string literal,
	//but it's the same as '[...]\[[:this]] and escape them like \\\[[:this]][...]'
	let mut template: Template = from_str("Hello, [[:name]]! Write placeholders like \\[[:this]] and escape them like \\\\\\[[:this]]").unwrap();

	//Insert something into the `name` placeholder
	//The ~(...) pattern is currently necessary because of how the compiler handles ~str
	template.insert("name", ~("Peter"));

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

fn main() {
	//Create a new Template from a string
	let mut template: Template = from_str("Hello, [[:name]]![[?condition]] The condition is true.[[/condition]]").unwrap();

	//Insert something into the `name` placeholder
	//The ~(...) pattern is currently necessary because of how the compiler handles ~str
	template.insert("name", ~("Peter"));

	//Conditions are false by default, so the second sentence will be disabled
	//Result: 'Hello, Peter!'
	println!("Result: '{}'", template);

	//Let's enable the hidden part of the template
	template.set("condition", true);

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

//This function will just concatenate the arguments.
//I expect you to make cooler generators, yourself ;)
fn join(parts: &[~str]) -> ~Show {
	~(parts.concat()) as ~Show
}


fn main() {
	//Create a new Template from a string
	let mut template: Template = from_str("Hello, [[:name]]! Is it written as 'white space' or '[[+join white space]]'?").unwrap();

	//Insert something into the `name` placeholder
	//The ~(...) pattern is currently necessary because of how the compiler handles ~str
	template.insert("name", ~("Peter"));

	//Functions with the signature `fn(&[~str]) -> ~Show` will automatically implement the `Generator` trait
	template.insert_generator("join", ~join);

	//Result: "Hello, Peter! Is it written as 'white space' or 'whitespace'?"
	println!("Result: '{}'", template);
}
```
