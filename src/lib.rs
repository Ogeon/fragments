#[crate_id = "fragments#0.1-pre"];

#[comment = "A simple template library for Rust"];
#[license = "MIT"];
#[crate_type = "lib"];
#[crate_type = "rlib"];

extern crate collections;

use std::fmt::Show;
use std::from_str::FromStr;
use std::io::BufReader;
use collections::hashmap::HashMap;

#[deriving(Eq, TotalEq)]
enum Token {
	String(~str),
	Placeholder(~str)
}

pub struct Template<'a> {
	content: ~HashMap<~str, &'a Show>,
	priv tokens: ~[Token]
}

impl<'a> Template<'a> {
	pub fn from_buffer(b: &mut Buffer) -> Template {
		let mut tokens = ~[];
		let mut current_token = ~"";
		let mut is_placeholder = false;

		loop {
			match b.read_char() {
				Ok('[') => {
					if !is_placeholder {
						match b.read_char() {
							Ok('[') => {
								tokens.push(String(current_token));
								current_token = ~"";
								is_placeholder = true;
							},
							Ok(c) => {
								current_token.push_char('[');
								current_token.push_char(c);
							},
							Err(_) => current_token.push_char('[')
						}
					} else {
						current_token.push_char('[')
					}
				},
				Ok(']') => {
					if is_placeholder {
						match b.read_char() {
							Ok(']') => {
								tokens.push(Placeholder(current_token));
								current_token = ~"";
								is_placeholder = false;
							},
							Ok(c) => {
								current_token.push_char(']');
								current_token.push_char(c);
							},
							Err(_) => current_token.push_char(']')
						}
					} else {
						current_token.push_char(']')
					}
				},
				Ok(c) => current_token.push_char(c),
				Err(_) => break
			}
		}

		if is_placeholder {
			tokens.push(Placeholder(current_token));
		} else {
			tokens.push(String(current_token));
		}

		Template {
			content: ~HashMap::new(),
			tokens: tokens
		}
	}
}

impl<'a> FromStr for Template<'a> {
	fn from_str(s: &str) -> Option<Template> {
		Some(Template::from_buffer(&mut BufReader::new(s.as_bytes())))
	}
}



#[cfg(test)]
mod test {
	use super::{Template, Placeholder, String};

	#[test]
	fn basic_tokens() {
		let template: Template = from_str("Hello, [[name]]! This is a [[something]] template.").unwrap();
		assert_eq!(template.tokens[0], String(~"Hello, "));
		assert_eq!(template.tokens[1], Placeholder(~"name"));
		assert_eq!(template.tokens[2], String(~"! This is a "));
		assert_eq!(template.tokens[3], Placeholder(~"something"));
		assert_eq!(template.tokens[4], String(~" template."));
	}

	#[test]
	fn strange_tokens() {
		let template: Template = from_str("Hello, [[[name]]]! This is a [[[[something]] template.").unwrap();
		assert_eq!(template.tokens[0], String(~"Hello, "));
		assert_eq!(template.tokens[1], Placeholder(~"[name"));
		assert_eq!(template.tokens[2], String(~"]! This is a "));
		assert_eq!(template.tokens[3], Placeholder(~"[[something"));
		assert_eq!(template.tokens[4], String(~" template."));
	}
}