#![crate_id = "fragments#0.1-pre"]

#![comment = "A simple template library for Rust"]
#![license = "MIT"]
#![crate_type = "lib"]
#![crate_type = "rlib"]

#![doc(html_root_url = "http://www.rust-ci.org/Ogeon/fragments/doc/")]

extern crate collections;

use std::fmt;
use std::from_str::FromStr;
use std::vec::Vec;
use collections::hashmap::{HashMap, HashSet};

mod parser;

#[deriving(Eq, TotalEq, Show)]
enum Token {
	String(~str),
	Placeholder(~str),
	Conditional(~str, bool, Vec<Token>),
	ContentConditional(~str, bool, Vec<Token>),
	Generated(~str, Vec<~str>)
}

///A string template with placeholders and conditional content.
///
///Placeholders are written as `[[:label]]`, where `label` becomes the name of the placeholder.
///The label is then used to insert content: `my_template.insert("label", my_content);`.
///The assigned content for a placeholder can be anything that implements `Show`.
///Even other templates may be inserted, which allows a more atomic structure.
///
///Conditional segments are surrounded by `[[?label]]...[[/]]`, where `label` becomes the name of the condition,
///and they are used to display content depending on whether its label exists in the `conditions` set.
///`[[/]]` marks the end of a block and may contain other characters after the `/`, which may be useful for labeling the end mark.
///Conditions can be made negative by writing `[[?!label]]...[[/]]`, which makes the content visible if the label
///is missing from the `conditions` set. Conditional segments can also depend on whether a placeholder has an assgined value. 
///Just write them like this: `[[?:label]]...[[/]]` or `[[?!:label]]...[[/]]`.
///
///Content can also be generated, using a generator token: `[[+label arg1 arg2 ...]]`. The label and the arguments are
///separated by one or more whitespaces. They can also be quoted to prevent special characters from being parsed:
///`[[+"my label" arg1 "[[arg2]]"]]`. The arguments will be passed to an instance of the `Generator` trait and the
///result will be inserted into the content.
///
///Any character can be escaped by writing `\` before it. It can be used like this: `\[[[:label1]], [[:label2]]]`
///which will result in `[content1, content2]`, since the first `[` will be ignored by the parser and added to the
///rest of the content.
pub struct Template {
	///Content for the placeholders
	pub content: HashMap<~str, ~fmt::Show: Send>,
	///Content generators
	pub generators: HashMap<~str, ~Generator: Send>,
	///Conditional switches
	pub conditions: HashSet<~str>,
	tokens: Vec<Token>
}

impl Template {
	///Create a new `Template` from a character iterator.
	#[inline]
	pub fn from_chars(b: &mut std::str::Chars) -> Result<Template, ~str> {
		let tokens = try!(parser::parse(&mut b.map(|r| Ok::<char, ~str>(r))));

		Ok(Template {
			content: HashMap::new(),
			generators: HashMap::new(),
			conditions: HashSet::new(),
			tokens: tokens
		})
	}

	///Create a new `Template` from a buffer.
	#[inline]
	pub fn from_buffer<T: Buffer>(b: &mut T) -> Result<Template, ~str> {
		let tokens = try!(parser::parse(&mut b.chars().map(|r| match r {
			Ok(c) => Ok(c),
			Err(e) => Err(format!("io error: {}", e))
		})));

		Ok(Template {
			content: HashMap::new(),
			generators: HashMap::new(),
			conditions: HashSet::new(),
			tokens: tokens
		})
	}

	///Convenience method for inserting content.
	#[inline]
	pub fn insert<T: fmt::Show + Send>(&mut self, label: &str, item: ~T) {
		self.content.insert(label.to_owned(), item as ~fmt::Show: Send);
	}

	///Convenience method for inserting generators.
	#[inline]
	pub fn insert_generator<T: Generator + Send>(&mut self, label: &str, gen: ~T) {
		self.generators.insert(label.to_owned(), gen as ~Generator: Send);
	}

	///Convenience method for setting a condition.
	#[inline]
	pub fn set(&mut self, label: &str, value: bool) {
		if value {
			self.conditions.insert(label.to_owned());
		} else {
			self.conditions.remove(&label.to_owned());
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
				},

				&ContentConditional(ref k, expected, ref tokens) => {
					if self.content.contains_key(k) == expected {
						self.format_tokens(tokens, f)
					} else {
						Ok(())
					}
				},

				&Generated(ref k, ref vars) => {
					match self.generators.find(k) {
						Some(gen) => gen.generate(vars.as_slice()).fmt(f),
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

impl FromStr for Template {
	///Creates a new `Template` from a string.
	fn from_str(s: &str) -> Option<Template> {
		match Template::from_chars(&mut s.chars()) {
			Ok(template) => Some(template),
			Err(_) => None
		}
	}
}

impl fmt::Show for Template {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.format_tokens(&self.tokens, f)
	}
}


///A trait for content generators.
pub trait Generator {
	fn generate(&self, args: &[~str]) -> ~fmt::Show;
}

impl Generator for fn(args: &[~str]) -> ~fmt::Show {
	fn generate(&self, args: &[~str]) -> ~fmt::Show {
		(*self)(args)
	}
}


#[cfg(test)]
mod test {
	use super::{Template, Placeholder, String};
	use std::fmt::Show;

	fn monitored_from_str(s: &str) -> Template {
		match Template::from_chars(&mut s.chars()) {
			Ok(template) => template,
			Err(e) => fail!(e)
		}
	}

	fn echo(parts: &[~str]) -> ~Show {
		~(parts.connect(":")) as ~Show
	}

	#[test]
	fn basic_tokens() {
		let template: Template = from_str("Hello, [[:name]]! This is a [[:something]] template.").unwrap();
		assert_eq!(template.tokens.get(0), &String("Hello, ".to_owned()));
		assert_eq!(template.tokens.get(1), &Placeholder("name".to_owned()));
		assert_eq!(template.tokens.get(2), &String("! This is a ".to_owned()));
		assert_eq!(template.tokens.get(3), &Placeholder("something".to_owned()));
		assert_eq!(template.tokens.get(4), &String(" template.".to_owned()));
	}

	#[test]
	#[should_fail]
	fn strange_tokens() {
		let _: Template = from_str("Hello, [[[:name]]]! This is a [[[[:something]] template.").unwrap();
	}

	#[test]
	fn escaped_tokens() {
		let template: Template = monitored_from_str("Hello, [[:name]]! Write placeholders like \\[[:this]] and escape them like \\\\\\[[:this]]");
		assert_eq!(template.tokens.get(0), &String("Hello, ".to_owned()));
		assert_eq!(template.tokens.get(1), &Placeholder("name".to_owned()));
		assert_eq!(template.tokens.get(2), &String("! Write placeholders like [[:this]] and escape them like \\[[:this]]".to_owned()));
	}

	#[test]
	fn replacement() {
		let mut template: Template = monitored_from_str("Hello, [[:name]]! This is a [[:something]] template.");
		template.insert("name", ~("Peter"));
		template.insert("something", ~("nice"));
		assert_eq!(template.to_str(), "Hello, Peter! This is a nice template.".to_owned());
	}

	#[test]
	fn templates_in_templates() {
		let mut template1: Template = monitored_from_str("Hello, [[:name]]! This is a [[:something]] template.");
		let mut template2: ~Template = ~monitored_from_str("really [[:something]]");
		template1.insert("name", ~("Peter"));
		template2.insert("something", ~("nice"));

		template1.insert("something", template2);

		assert_eq!(template1.to_str(), "Hello, Peter! This is a really nice template.".to_owned());
	}

	#[test]
	fn conditional() {
		let mut template: Template = monitored_from_str("Hello, [[:name]]![[?condition]] The condition is true.[[/condition]]");
		template.insert("name", ~("Peter"));
		assert_eq!(template.to_str(), "Hello, Peter!".to_owned());
		template.set("condition", true);
		assert_eq!(template.to_str(), "Hello, Peter! The condition is true.".to_owned());
	}

	#[test]
	fn conditional_switch() {
		let mut template: Template = monitored_from_str("Hello, [[:name]]! The condition is [[?condition]]true[[/condition]][[?!condition]]false[[/condition]].");
		template.insert("name", ~("Peter"));
		assert_eq!(template.to_str(), "Hello, Peter! The condition is false.".to_owned());
		template.set("condition", true);
		assert_eq!(template.to_str(), "Hello, Peter! The condition is true.".to_owned());
	}

	#[test]
	fn content_conditional() {
		let mut template: Template = monitored_from_str("Hello[[?:name]], [[:name]][[/name]]![[?!:name]] I don't know you.[[/!name]]");
		assert_eq!(template.to_str(), "Hello! I don't know you.".to_owned());
		template.insert("name", ~("Peter"));
		assert_eq!(template.to_str(), "Hello, Peter!".to_owned());
	}

	#[test]
	fn generator() {
		let mut template: Template = monitored_from_str("[[+\"say hello\" hello Peter    \"how are\" you?]]");
		template.insert_generator("say hello", ~echo);

		assert_eq!(template.to_str(), "hello:Peter:how are:you?".to_owned());
	}
}