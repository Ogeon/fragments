#[crate_id = "fragments#0.1-pre"];

#[comment = "A simple template library for Rust"];
#[license = "MIT"];
#[crate_type = "lib"];
#[crate_type = "rlib"];

extern crate collections;

use std::fmt::Show;
use std::from_str::FromStr;
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

impl<'a> FromStr for Template<'a> {
	fn from_str(s: &str) -> Option<Template> {
		let mut tokens = ~[];
		let mut current_token = ~"";
		let mut is_placeholder = false;

		let mut chars = s.chars();

		loop {
			match chars.next() {
				Some('[') => {
					if !is_placeholder {
						match chars.next() {
							Some('[') => {
								tokens.push(String(current_token));
								current_token = ~"";
								is_placeholder = true;
							},
							Some(c) => {
								current_token.push_char('[');
								current_token.push_char(c);
							},
							None => current_token.push_char('[')
						}
					} else {
						current_token.push_char('[')
					}
				},
				Some(']') => {
					if is_placeholder {
						match chars.next() {
							Some(']') => {
								tokens.push(Placeholder(current_token));
								current_token = ~"";
								is_placeholder = false;
							},
							Some(c) => {
								current_token.push_char(']');
								current_token.push_char(c);
							},
							None => current_token.push_char(']')
						}
					} else {
						current_token.push_char(']')
					}
				},
				Some(c) => current_token.push_char(c),
				None => break
			}
		}

		if is_placeholder {
			tokens.push(Placeholder(current_token));
		} else {
			tokens.push(String(current_token));
		}

		Some(Template {
			content: ~HashMap::new(),
			tokens: tokens
		})
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