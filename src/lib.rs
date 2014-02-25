#[crate_id = "fragments#0.1-pre"];

#[comment = "A simple template library for Rust"];
#[license = "MIT"];
#[crate_type = "lib"];
#[crate_type = "rlib"];

extern crate collections;

use std::fmt;
use std::from_str::FromStr;
use std::io::BufReader;
use collections::hashmap::HashMap;

#[deriving(Eq, TotalEq)]
enum Token {
	String(~str),
	Placeholder(~str)
}

pub struct Template {
	content: ~HashMap<~str, ~fmt::Show>,
	priv tokens: ~[Token]
}

impl Template {
	pub fn from_buffer(b: &mut Buffer) -> Template {
		let mut tokens = ~[];
		let mut current_token = ~"";
		let mut is_placeholder = false;

		loop {
			match b.read_char() {
				Ok('\\') => {
					match b.read_char() {
						Ok(c) => {
							current_token.push_char(c);
						},
						_ => break
					}
				},
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

	pub fn insert<T: fmt::Show + Send>(&mut self, placeholder: ~str, item: ~T) {
		self.content.insert(placeholder, item as ~fmt::Show);
	}
}

impl FromStr for Template {
	fn from_str(s: &str) -> Option<Template> {
		Some(Template::from_buffer(&mut BufReader::new(s.as_bytes())))
	}
}

impl fmt::Show for Template {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		for token in self.tokens.iter() {
			let res = match token {
				&String(ref s) => f.buf.write_str(*s),

				&Placeholder(ref k) => {
					match self.content.find(k) {
						Some(value) => value.fmt(f),
						None => Ok(())
					}
				}
			};

			match res {
				Err(e) => return Err(e),
				_ => {}
			}
		}

		Ok(())
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

	#[test]
	fn escaped_tokens() {
		let template: Template = from_str("Hello, [[name]]! Write placeholders like \\[[this]] and escape them like \\\\\\[[this]]").unwrap();
		assert_eq!(template.tokens[0], String(~"Hello, "));
		assert_eq!(template.tokens[1], Placeholder(~"name"));
		assert_eq!(template.tokens[2], String(~"! Write placeholders like [[this]] and escape them like \\[[this]]"));
	}

	#[test]
	fn replacement() {
		let mut template: Template = from_str("Hello, [[name]]! This is a [[something]] template.").unwrap();
		template.insert(~"name", ~("Peter"));
		template.insert(~"something", ~("nice"));
		assert_eq!(template.to_str(), ~"Hello, Peter! This is a nice template.");
	}

	#[test]
	fn templates_in_templates() {
		let mut template1: Template = from_str("Hello, [[name]]! This is a [[something]] template.").unwrap();
		let mut template2: ~Template = ~from_str("really [[something]]").unwrap();
		template1.insert(~"name", ~("Peter"));
		template2.insert(~"something", ~("nice"));

		template1.insert(~"something", template2);

		assert_eq!(template1.to_str(), ~"Hello, Peter! This is a really nice template.");
	}
}