#![crate_id = "fragments#0.1-pre"]

#![comment = "A simple template library for Rust"]
#![license = "MIT"]
#![crate_type = "lib"]
#![crate_type = "rlib"]

#![doc(html_root_url = "http://ogeon.github.io/fragments/doc/")]

#![feature(macro_rules)]

extern crate collections;

use std::fmt;
use std::fmt::Show;
use std::from_str::FromStr;
use std::vec::Vec;
use collections::hashmap::{HashMap, HashSet};

mod parser;

#[deriving(Eq, TotalEq, Show)]
enum Token {
	String(String),
	Placeholder(String),
	Conditional(String, bool, Vec<Token>),
	ContentConditional(String, bool, Vec<Token>),
	Generated(String, Vec<String>)
}

///Container enum for template content
pub enum ContentType {
	Float(f64, Option<uint>), ///A float with an optional precision
	Int(i64),
	UnsignedInt(u64),
	Char(char),
	Bool(bool),
	Str(String),
	StaticStr(&'static str),
	Template(Template),
	Show(Box<fmt::Show>)
}

macro_rules! call_fmt(
	($($p:pat => $b:expr),+ and $($t:ident),+) => (
		match self {
			$($p => $b,)+
			$(&$t(ref v) => v.fmt(f)),+
		}
	)
)

impl fmt::Show for ContentType {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		call_fmt! {
			&Float(v, p) => {
				let prev_p = f.precision;
				f.precision = p;
				let result = (&v as &fmt::Float).fmt(f);
				f.precision = prev_p;
				result
			}
			
			and

			Int,
			UnsignedInt,
			Char,
			Bool,
			Str,
			StaticStr,
			Template,
			Show
		}
	}
}


///A trait for types that can be inserted into templates
pub trait TemplateContent {
	///Convert `self` to a suitable `ContentType` variant, not making a copy if possible.
	fn into_template_content(self) -> ContentType;
}

///A trait for types that can be copied and inserted into templates
pub trait CopyTemplateContent {
	///Copy and convert `self` to a suitable `ContentType` variant.
	fn to_template_content(&self) -> ContentType;
}

macro_rules! float_content(
	($($t:ty),+) => (
		$(impl TemplateContent for $t {
			fn into_template_content(self) -> ContentType {
				Float(self as f64, None)
			}
		}

		impl CopyTemplateContent for $t {
			fn to_template_content(&self) -> ContentType {
				Float(*self as f64, None)
			}
		})+
	)
)

macro_rules! int_content(
	($($t:ty),+) => (
		$(impl TemplateContent for $t {
			fn into_template_content(self) -> ContentType {
				Int(self as i64)
			}
		}

		impl CopyTemplateContent for $t {
			fn to_template_content(&self) -> ContentType {
				Int(*self as i64)
			}
		})+
	)
)

macro_rules! uint_content(
	($($t:ty),+) => (
		$(impl TemplateContent for $t {
			fn into_template_content(self) -> ContentType {
				UnsignedInt(self as u64)
			}
		}

		impl CopyTemplateContent for $t {
			fn to_template_content(&self) -> ContentType {
				UnsignedInt(*self as u64)
			}
		})+
	)
)

macro_rules! deref_content(
	($([$t:ty, $i:ident]),+) => (
		$(impl TemplateContent for $t {
			fn into_template_content(self) -> ContentType {
				$i(self)
			}
		}

		impl CopyTemplateContent for $t {
			fn to_template_content(&self) -> ContentType {
				$i(*self)
			}
		})+
	)
)

float_content!(f32, f64)
int_content!(int, i8, i16, i32, i64)
uint_content!(uint, u8, u16, u32, u64)
deref_content!([char, Char], [bool, Bool], [&'static str, StaticStr])


impl TemplateContent for String {
	fn into_template_content(self) -> ContentType {
		Str(self)
	}
}

impl CopyTemplateContent for String {
	fn to_template_content(&self) -> ContentType {
		Str(self.clone())
	}
}


impl TemplateContent for Template {
	fn into_template_content(self) -> ContentType {
		Template(self)
	}
}


impl TemplateContent for Box<Show> {
	fn into_template_content(self) -> ContentType {
		Show(self)
	}
}


impl TemplateContent for ContentType {
	fn into_template_content(self) -> ContentType {
		self
	}
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
///is missing from the `conditions` set. Conditional segments can also depend on whether a placeholder has an assigned value. 
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
	pub content: HashMap<String, ContentType>,
	///Content generators
	pub generators: HashMap<String, Box<Generator>>,
	///Conditional switches
	pub conditions: HashSet<String>,
	tokens: Vec<Token>
}

impl Template {
	///Create a new `Template` from a character iterator.
	#[inline]
	pub fn from_chars(b: &mut std::str::Chars) -> Result<Template, String> {
		let tokens = try!(parser::parse(&mut b.map(|r| Ok::<char, String>(r))));

		Ok(Template {
			content: HashMap::new(),
			generators: HashMap::new(),
			conditions: HashSet::new(),
			tokens: tokens
		})
	}

	///Create a new `Template` from a buffer.
	#[inline]
	pub fn from_buffer<T: Buffer>(b: &mut T) -> Result<Template, String> {
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

	///Insert content.
	#[inline]
	pub fn insert<S: StrAllocating, T: TemplateContent>(&mut self, label: S, item: T) {
		self.content.insert(label.into_string(), item.into_template_content());
	}

	///Insert a formatted float.
	#[inline]
	pub fn insert_float<S: StrAllocating>(&mut self, label: S, item: f64, precision: uint) {
		self.content.insert(label.into_string(), Float(item, Some(precision)));
	}

	///Insert a content generator.
	#[inline]
	pub fn insert_generator<S: StrAllocating, T: Generator + Send>(&mut self, label: S, gen: T) {
		self.generators.insert(label.into_string(), box gen as Box<Generator>);
	}

	///Set a condition.
	#[inline]
	pub fn set<S: StrAllocating>(&mut self, label: S, value: bool) {
		if value {
			self.conditions.insert(label.into_string());
		} else {
			self.conditions.remove(&label.into_string());
		}
	}

	fn format_tokens(&self, tokens: &Vec<Token>, f: &mut fmt::Formatter) -> fmt::Result {
		for token in tokens.iter() {
			let res = match token {
				&String(ref s) => f.write(s.as_bytes()),

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
						Some(gen) => gen.generate(vars).fmt(f),
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
	fn generate(&self, args: &Vec<String>) -> Box<fmt::Show>;
}

impl Generator for fn(args: &Vec<String>) -> Box<fmt::Show> {
	fn generate(&self, args: &Vec<String>) -> Box<fmt::Show> {
		(*self)(args)
	}
}


#[cfg(test)]
mod test {
	use super::{Template, Placeholder, String};
	use std::fmt::Show;

	macro_rules! test_insert(
		($($v:expr),+) => (
			#[test]
			fn test_insert() {
				let mut template: Template = monitored_from_str("[[:v]]");
				$(
					template.insert("v", $v);
					assert_eq!(template.to_str(), $v.to_str());
				)+
			}
		)
	)

	static peter: &'static str = "Peter";
	static nice: &'static str = "nice";

	fn monitored_from_str(s: &str) -> Template {
		match Template::from_chars(&mut s.chars()) {
			Ok(template) => template,
			Err(e) => fail!(e)
		}
	}

	fn echo(parts: &Vec<String>) -> Box<Show> {
		box parts.connect(":") as Box<Show>
	}

	#[test]
	fn basic_tokens() {
		let template: Template = from_str("Hello, [[:name]]! This is a [[:something]] template.").unwrap();
		assert_eq!(template.tokens.get(0), &String("Hello, ".into_string()));
		assert_eq!(template.tokens.get(1), &Placeholder("name".into_string()));
		assert_eq!(template.tokens.get(2), &String("! This is a ".into_string()));
		assert_eq!(template.tokens.get(3), &Placeholder("something".into_string()));
		assert_eq!(template.tokens.get(4), &String(" template.".into_string()));
	}

	#[test]
	#[should_fail]
	fn strange_tokens() {
		let _: Template = from_str("Hello, [[[:name]]]! This is a [[[[:something]] template.").unwrap();
	}

	#[test]
	fn escaped_tokens() {
		let template = monitored_from_str("Hello, [[:name]]! Write placeholders like \\[[:this]] and escape them like \\\\\\[[:this]]");
		assert_eq!(template.tokens.get(0), &String("Hello, ".into_string()));
		assert_eq!(template.tokens.get(1), &Placeholder("name".into_string()));
		assert_eq!(template.tokens.get(2), &String("! Write placeholders like [[:this]] and escape them like \\[[:this]]".into_string()));
	}

	#[test]
	fn replacement() {
		let mut template = monitored_from_str("Hello, [[:name]]! This is a [[:something]] template.");
		template.insert("name", peter);
		template.insert("something", nice);
		assert_eq!(template.to_str(), "Hello, Peter! This is a nice template.".into_string());
	}

	#[test]
	fn templates_in_templates() {
		let mut template1 = monitored_from_str("Hello, [[:name]]! This is a [[:something]] template.");
		let mut template2 = monitored_from_str("really [[:something]]");
		template1.insert("name", peter);
		template2.insert("something", nice);

		template1.insert("something", template2);

		assert_eq!(template1.to_str(), "Hello, Peter! This is a really nice template.".into_string());
	}

	#[test]
	fn conditional() {
		let mut template = monitored_from_str("Hello, [[:name]]![[?condition]] The condition is true.[[/condition]]");
		template.insert("name", peter);
		assert_eq!(template.to_str(), "Hello, Peter!".into_string());
		template.set("condition", true);
		assert_eq!(template.to_str(), "Hello, Peter! The condition is true.".into_string());
	}

	#[test]
	fn conditional_switch() {
		let mut template = monitored_from_str("Hello, [[:name]]! The condition is [[?condition]]true[[/condition]][[?!condition]]false[[/condition]].");
		template.insert("name", peter);
		assert_eq!(template.to_str(), "Hello, Peter! The condition is false.".into_string());
		template.set("condition", true);
		assert_eq!(template.to_str(), "Hello, Peter! The condition is true.".into_string());
	}

	#[test]
	fn content_conditional() {
		let mut template = monitored_from_str("Hello[[?:name]], [[:name]][[/name]]![[?!:name]] I don't know you.[[/!name]]");
		assert_eq!(template.to_str(), "Hello! I don't know you.".into_string());
		template.insert("name", peter);
		assert_eq!(template.to_str(), "Hello, Peter!".into_string());
	}

	#[test]
	fn generator() {
		let mut template = monitored_from_str("[[+\"say hello\" hello Peter    \"how are\" you?]]");
		template.insert_generator("say hello", echo);

		assert_eq!(template.to_str(), "hello:Peter:how are:you?".into_string());
	}

	#[test]
	fn format_float() {
		let mut template = monitored_from_str("[[:short]], [[:long]], [[:default]]");
		template.insert_float("short", 1.2, 1);
		template.insert_float("long", 1.2, 4);
		template.insert("default", 1.2);
		assert_eq!(template.to_str(), "1.2, 1.2000, 1.2".into_string())
	}

	test_insert!(1u8, 1u16, 1u32, 1u64, 1i8, 1i16, 1i32, 1i64, 'A', true, false)
}