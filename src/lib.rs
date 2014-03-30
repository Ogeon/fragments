#![crate_id = "fragments#0.1-pre"]

#![comment = "A simple template library for Rust"]
#![license = "MIT"]
#![crate_type = "lib"]
#![crate_type = "rlib"]

#![doc(html_root_url = "http://www.rust-ci.org/Ogeon/fragments/doc/")]

extern crate collections;

use std::fmt;
use std::from_str::FromStr;
use std::io::{BufReader, IoError, EndOfFile};
use std::vec::Vec;
use collections::hashmap::{HashMap, HashSet};

type ParserState = Option<fn(&mut Buffer) -> ParserResult>;

struct ParserResult {
	token: Token,
	next_parser: ParserState
}

#[deriving(Eq, TotalEq, Show)]
enum Token {
	String(~str),
	Placeholder(~str),
	Conditional(~str, bool, Vec<Token>)
}

///A string template with placeholders and conditional content.
///
///Placeholders are written as `[[:label]]`, where `label` becomes the name of the placeholder.
///The label is then used to insert content: `my_template.insert(~"label", my_content);`.
///The assigned content for a placeholder can be anything that implements `Show`.
///Even other templates may be inserted, which allows a more atomic structure.
///
///Conditional segments are surrounded by `[[?label]]...[[/]]`, where `label` becomes the name of the condition,
///and they are used to display content depending on whether its label exists in the `conditions` set.
///`[[/]]` marks the end of a block and may contain other characters after the `/`, which may be useful for labeling the end mark.
///Conditions can be made negative by writing `[[?!label]]...[[/]]`, which makes the content visible if the label
///is missing from the `conditions` set.
///
///Any character can be escaped by writing `\` before it. It can be used like this: `\[[[:label1]], [[:label2]]]`
///which will result in `[content1, content2]`, since the first `[` will be ignored by the parser and just added to the
///rest of the content.
pub struct Template {
	///Content for the placeholders
	content: HashMap<~str, ~fmt::Show: Send>,
	///Conditional switches
	conditions: HashSet<~str>,
	priv tokens: Vec<Token>
}

impl Template {
	#[inline]
	pub fn from_buffer(b: &mut Buffer) -> Template {
		Template {
			content: HashMap::new(),
			conditions: HashSet::new(),
			tokens: parse_block(b)
		}
	}

	///Convenience method for inserting content.
	#[inline]
	pub fn insert<T: fmt::Show + Send>(&mut self, placeholder: ~str, item: ~T) {
		self.content.insert(placeholder, item as ~fmt::Show: Send);
	}

	///Convenience method for setting a condition.
	#[inline]
	pub fn set(&mut self, condition: ~str, value: bool) {
		if value {
			self.conditions.insert(condition);
		} else {
			self.conditions.remove(&condition);
		}
	}

	fn format_tokens(&self, tokens: &Vec<Token>, f: &mut fmt::Formatter) -> fmt::Result {
		for token in tokens.iter() {
			let res = match token {
				&String(ref s) => f.buf.write_str(*s),

				&Placeholder(ref k) => {
					match self.content.find(k) {
						Some(value) => value.fmt(f),
						None => Ok(())
					}
				},

				&Conditional(ref k, expected, ref tokens) => {
					if self.conditions.contains(k) == expected {
						self.format_tokens(tokens, f)
					} else {
						Ok(())
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

impl FromStr for Template {
	///Creates a new `Template` from a string.
	fn from_str(s: &str) -> Option<Template> {
		Some(Template::from_buffer(&mut BufReader::new(s.as_bytes())))
	}
}

impl fmt::Show for Template {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.format_tokens(&self.tokens, f)
	}
}

fn parse_block(b: &mut Buffer) -> Vec<Token> {
	let mut tokens = vec!();
	let mut parser = parse_string;

	loop {
		let ParserResult{token: token, next_parser: next} = parser(b);

		tokens.push(token);

		match next {
			Some(p) => parser = p,
			None => break
		}
	}

	tokens
}

fn parse_string(b: &mut Buffer) -> ParserResult {
	let mut content = ~"";

	loop {
		match b.read_char() {
			Ok('\\') => {
				match b.read_char() {
					Ok(c) => {
						content.push_char(c);
					},
					Err(IoError{kind: EndOfFile, desc: _, detail: _}) => break,
					Err(e) => fail!("{}", e)
				}
			},
			Ok('[') => {
				match b.read_char() {
					Ok('[') => {
						match b.read_char() {
							Ok(':') => return ParserResult{
								token: String(content),
								next_parser: Some(parse_placeholder)
							},
							Ok('?') => return ParserResult{
								token: String(content),
								next_parser: Some(parse_conditional)
							},
							Ok('/') => {
								skip_to_token_end(b);

								return ParserResult{
									token: String(content),
									next_parser: None
								}
							},
							Ok(c) => fail!("Unknown token type: '{}'", c),
							Err(IoError{kind: EndOfFile, desc: _, detail: _}) => break,
							Err(e) => fail!("{}", e)
						}
					},
					Ok(c) => {
						content.push_char('[');
						content.push_char(c);
					},
					Err(IoError{kind: EndOfFile, desc: _, detail: _}) => content.push_char('['),
					Err(e) => fail!("{}", e)
				}
			},
			Ok(c) => content.push_char(c),
			Err(IoError{kind: EndOfFile, desc: _, detail: _}) => break,
			Err(e) => fail!("{}", e)
		}
	}

	ParserResult{
		token: String(content),
		next_parser: None
	}
}

fn skip_to_token_end(b: &mut Buffer) {
	loop {
		match b.read_char() {
			Ok('\\') => {
				match b.read_char() {
					Ok(_) => {},
					Err(IoError{kind: EndOfFile, desc: _, detail: _}) => break,
					Err(e) => fail!("{}", e)
				}
			},
			Ok(']') => {
				match b.read_char() {
					Ok(']') => break,
					Err(IoError{kind: EndOfFile, desc: _, detail: _}) => break,
					Err(e) => fail!("{}", e),
					_ => {}
				}
			},
			Err(IoError{kind: EndOfFile, desc: _, detail: _}) => break,
			Err(e) => fail!("{}", e),
			_ => {}
		}
	}
}

fn parse_placeholder(b: &mut Buffer) -> ParserResult {
	let mut label = ~"";

	loop {
		match b.read_char() {
			Ok('\\') => {
				match b.read_char() {
					Ok(c) => {
						label.push_char(c);
					},
					Err(IoError{kind: EndOfFile, desc: _, detail: _}) => break,
					Err(e) => fail!("{}", e)
				}
			},
			Ok(']') => {
				match b.read_char() {
					Ok(']') => {
						return ParserResult{
							token: Placeholder(label),
							next_parser: Some(parse_string)
						}
					},
					Ok(c) => {
						label.push_char(']');
						label.push_char(c);
					},
					Err(IoError{kind: EndOfFile, desc: _, detail: _}) => label.push_char(']'),
					Err(e) => fail!("{}", e)
				}
			},
			Ok(c) => label.push_char(c),
			Err(IoError{kind: EndOfFile, desc: _, detail: _}) => break,
			Err(e) => fail!("{}", e)
		}
	}

	ParserResult{
		token: Placeholder(label),
		next_parser: None
	}
}

fn parse_conditional(b: &mut Buffer) -> ParserResult {
	let mut label = ~"";
	let mut expected = true;
	let mut expected_set = false;

	loop {
		match b.read_char() {
			Ok('\\') => {
				match b.read_char() {
					Ok(c) => {
						label.push_char(c);
					},
					Err(IoError{kind: EndOfFile, desc: _, detail: _}) => break,
					Err(e) => fail!("{}", e)
				}
			},
			Ok(']') => {
				match b.read_char() {
					Ok(']') => {
						return ParserResult{
							token: Conditional(label, expected, parse_block(b)),
							next_parser: Some(parse_string)
						}
					},
					Ok(c) => {
						label.push_char(']');
						label.push_char(c);
					},
					Err(IoError{kind: EndOfFile, desc: _, detail: _}) => label.push_char(']'),
					Err(e) => fail!("{}", e)
				}
			},
			Ok('!') => {
				if label.len() == 0 && !expected_set {
					expected = false;
					expected_set = true;
				} else {
					label.push_char('!');
				}
			},
			Ok(c) => {
				label.push_char(c)
			},
			Err(IoError{kind: EndOfFile, desc: _, detail: _}) => break,
			Err(e) => fail!("{}", e)
		}
	}

	ParserResult{
		token: Conditional(label, expected, vec!()),
		next_parser: None
	}
}


#[cfg(test)]
mod test {
	use super::{Template, Placeholder, String};

	#[test]
	fn basic_tokens() {
		let template: Template = from_str("Hello, [[:name]]! This is a [[:something]] template.").unwrap();
		assert_eq!(template.tokens.get(0), &String(~"Hello, "));
		assert_eq!(template.tokens.get(1), &Placeholder(~"name"));
		assert_eq!(template.tokens.get(2), &String(~"! This is a "));
		assert_eq!(template.tokens.get(3), &Placeholder(~"something"));
		assert_eq!(template.tokens.get(4), &String(~" template."));
	}

	#[test]
	#[should_fail]
	fn strange_tokens() {
		let _: Template = from_str("Hello, [[[:name]]]! This is a [[[[:something]] template.").unwrap();
	}

	#[test]
	fn escaped_tokens() {
		let template: Template = from_str("Hello, [[:name]]! Write placeholders like \\[[:this]] and escape them like \\\\\\[[:this]]").unwrap();
		assert_eq!(template.tokens.get(0), &String(~"Hello, "));
		assert_eq!(template.tokens.get(1), &Placeholder(~"name"));
		assert_eq!(template.tokens.get(2), &String(~"! Write placeholders like [[:this]] and escape them like \\[[:this]]"));
	}

	#[test]
	fn replacement() {
		let mut template: Template = from_str("Hello, [[:name]]! This is a [[:something]] template.").unwrap();
		template.insert(~"name", ~("Peter"));
		template.insert(~"something", ~("nice"));
		assert_eq!(template.to_str(), ~"Hello, Peter! This is a nice template.");
	}

	#[test]
	fn templates_in_templates() {
		let mut template1: Template = from_str("Hello, [[:name]]! This is a [[:something]] template.").unwrap();
		let mut template2: ~Template = ~from_str("really [[:something]]").unwrap();
		template1.insert(~"name", ~("Peter"));
		template2.insert(~"something", ~("nice"));

		template1.insert(~"something", template2);

		assert_eq!(template1.to_str(), ~"Hello, Peter! This is a really nice template.");
	}

	#[test]
	fn conditional() {
		let mut template: Template = from_str("Hello, [[:name]]![[?condition]] The condition is true.[[/condition]]").unwrap();
		template.insert(~"name", ~("Peter"));
		assert_eq!(template.to_str(), ~"Hello, Peter!");
		template.set(~"condition", true);
		assert_eq!(template.to_str(), ~"Hello, Peter! The condition is true.");
	}

	#[test]
	fn conditional_switch() {
		let mut template: Template = from_str("Hello, [[:name]]! The condition is [[?condition]]true[[/condition]][[?!condition]]false[[/condition]].").unwrap();
		template.insert(~"name", ~("Peter"));
		assert_eq!(template.to_str(), ~"Hello, Peter! The condition is false.");
		template.set(~"condition", true);
		assert_eq!(template.to_str(), ~"Hello, Peter! The condition is true.");
	}
}