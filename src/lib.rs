#![crate_name = "fragments"]

#![comment = "A simple template library for Rust"]
#![license = "MIT"]
#![crate_type = "lib"]
#![crate_type = "rlib"]

#![doc(html_root_url = "http://ogeon.github.io/fragments/doc/")]

#![feature(macro_rules)]

use std::fmt;
use std::fmt::Show;
use std::str::FromStr;
use std::vec::Vec;
use std::collections::{HashMap, HashSet};
use std::borrow::ToOwned;
use std::num::strconv::{
	float_to_str_bytes_common,
	SignFormat
};

pub use std::num::strconv::{
	SignificantDigits,
	ExponentFormat
};

mod parser;

///Internal representation of template parts.
#[deriving(PartialEq, Show)]
pub enum Token {
	String(String),
	Placeholder(String),
	Conditional(String, bool, Vec<Token>),
	ContentConditional(String, bool, Vec<Token>),
	Generated(String, Vec<String>)
}

///Container enum for template content
pub enum ContentType<'a> {
	Float(f64),
	FormattedFloat(f64, SignificantDigits, ExponentFormat),
	Int(i64),
	UnsignedInt(u64),
	Char(char),
	Bool(bool),
	Str(String),
	StaticStr(&'static str),
	Template(Template),
	Show(Box<fmt::Show + 'a>)
}

macro_rules! call_fmt {
	($slf:ident, $f:ident: $($p:pat => $b:expr),+ and $($t:ident),+) => {
		match $slf {
			$($p => $b,)+
			$(&ContentType::$t(ref v) => v.fmt($f)),+
		}
	}
}

impl<'a> fmt::Show for ContentType<'a> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		call_fmt! {
			self,
			f:
			&ContentType::FormattedFloat(v, sig, exp) => {
				let (string, _special) = float_to_str_bytes_common(v, 10, false, SignFormat::SignNeg, sig, exp, false);
				string.fmt(f)
			}
			
			and

			Int,
			UnsignedInt,
			Float,
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
pub trait TemplateContent<'a> {
	///Convert `self` to a suitable `ContentType` variant, not making a copy if possible.
	fn into_template_content(self) -> ContentType<'a>;
}

///A trait for types that can be copied and inserted into templates
pub trait CopyTemplateContent<'a> {
	///Copy and convert `self` to a suitable `ContentType` variant.
	fn to_template_content(&self) -> ContentType;
}

macro_rules! float_content {
	($($t:ty),+) => {
		$(impl TemplateContent<'static> for $t {
			fn into_template_content(self) -> ContentType<'static> {
				ContentType::Float(self as f64)
			}
		}

		impl CopyTemplateContent<'static> for $t {
			fn to_template_content(&self) -> ContentType<'static> {
				ContentType::Float(*self as f64)
			}
		})+
	}
}

macro_rules! int_content {
	($($t:ty),+) => {
		$(impl TemplateContent<'static> for $t {
			fn into_template_content(self) -> ContentType<'static> {
				ContentType::Int(self as i64)
			}
		}

		impl CopyTemplateContent<'static> for $t {
			fn to_template_content(&self) -> ContentType<'static> {
				ContentType::Int(*self as i64)
			}
		})+
	}
}

macro_rules! uint_content {
	($($t:ty),+) => {
		$(impl TemplateContent<'static> for $t {
			fn into_template_content(self) -> ContentType<'static> {
				ContentType::UnsignedInt(self as u64)
			}
		}

		impl CopyTemplateContent<'static> for $t {
			fn to_template_content(&self) -> ContentType<'static> {
				ContentType::UnsignedInt(*self as u64)
			}
		})+
	}
}

macro_rules! deref_content {
	($([$t:ty, $i:ident]),+) => {
		$(impl TemplateContent<'static> for $t {
			fn into_template_content(self) -> ContentType<'static> {
				ContentType::$i(self)
			}
		}

		impl CopyTemplateContent<'static> for $t {
			fn to_template_content(&self) -> ContentType<'static> {
				ContentType::$i(*self)
			}
		})+
	}
}

float_content!(f32, f64);
int_content!(int, i8, i16, i32, i64);
uint_content!(uint, u8, u16, u32, u64);
deref_content!([char, Char], [bool, Bool], [&'static str, StaticStr]);


impl TemplateContent<'static> for String {
	fn into_template_content(self) -> ContentType<'static> {
		ContentType::Str(self)
	}
}

impl CopyTemplateContent<'static> for String {
	fn to_template_content(&self) -> ContentType<'static> {
		ContentType::Str(self.clone())
	}
}


impl TemplateContent<'static> for Template {
	fn into_template_content(self) -> ContentType<'static> {
		ContentType::Template(self)
	}
}


impl<'a> TemplateContent<'a> for Box<Show + 'a> {
	fn into_template_content(self) -> ContentType<'a> {
		ContentType::Show(self)
	}
}


impl<'a> TemplateContent<'a> for ContentType<'a> {
	fn into_template_content(self) -> ContentType<'a> {
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
	pub content: HashMap<String, ContentType<'static>>,
	///Content generators
	pub generators: HashMap<String, Box<Generator + 'static>>,
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
	pub fn insert<S: ToOwned<String> + ?Sized, T: TemplateContent<'static>>(&mut self, label: &S, item: T) {
		self.content.insert(label.to_owned(), item.into_template_content());
	}

	///Insert a formatted float.
	#[inline]
	pub fn insert_formatted_float<S: ToOwned<String> + ?Sized>(&mut self, label: &S, item: f64, precision: SignificantDigits, exponent: ExponentFormat) {
		self.content.insert(label.to_owned(), ContentType::FormattedFloat(item, precision, exponent));
	}

	///Insert a content generator.
	#[inline]
	pub fn insert_generator<S: ToOwned<String> + ?Sized, T: Generator + Send>(&mut self, label: &S, gen: T) {
		self.generators.insert(label.to_owned(), box gen as Box<Generator>);
	}

	///Set a condition.
	#[inline]
	pub fn set<S: ToOwned<String> + ?Sized + Eq + std::hash::Hash>(&mut self, label: &S, value: bool) {
		if value {
			self.conditions.insert(label.to_owned());
		} else {
			self.conditions.remove(label);
		}
	}

	///Create a `Shell` around this `Template`.
	#[inline]
	pub fn wrap<'a, 'c>(&'a self) -> Shell<'a, 'c> {
		Shell::new(self)
	}
}

impl InnerTemplate<'static> for Template {
	fn find_content<'a>(&'a self, label: &String) -> Option<&'a ContentType<'static>> {
		self.content.find(label)
	}

	fn get_condition(&self, label: &String) -> bool {
		self.conditions.contains(label)
	}
	
	fn is_content_definded(&self, label: &String) -> bool {
		self.content.contains_key(label)
	}

	fn find_generator<'a>(&'a self, label: &String) -> Option<&'a Box<Generator + 'static>> {
		self.generators.find(label)
	}

	fn get_tokens<'a>(&'a self) -> &'a Vec<Token> {
		&self.tokens
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
		format_tokens(self as &InnerTemplate, &self.tokens, f)
	}
}





///A `Shell` is a layer on top of an overridable template.
///
///Shells can be used to assign different values to a template and still
///keep the original intact.
pub struct Shell<'r, 'c: 'r> {
    ///Content for the placeholders
	pub content: HashMap<String, Option<ContentType<'r>>>,
	///Content generators
	pub generators: HashMap<String, Option<Box<Generator + 'r>>>,
	///Conditional switches
	pub conditions: HashMap<String, bool>,
    base: &'r (InnerTemplate<'c> + 'r)
}

impl<'r, 'c> Shell<'r, 'c> {
	///Create a new `Shell` around `base`.
	pub fn new<'a, 'b, T: InnerTemplate<'b>>(base: &'a T) -> Shell<'a, 'b> {
		Shell {
			content: HashMap::new(),
			generators: HashMap::new(),
			conditions: HashMap::new(),
			base: base as &InnerTemplate<'b>
		}
	}

	///Insert content.
	#[inline]
	pub fn insert<S: ToOwned<String> + ?Sized, T: TemplateContent<'c>>(&mut self, label: &S, item: T) {
		self.content.insert(label.to_owned(), Some(item.into_template_content()));
	}

	///Unset content.
	#[inline]
	pub fn unset<S: ToOwned<String> + ?Sized>(&mut self, label: &S) {
		self.content.insert(label.to_owned(), None);
	}

	///Insert a formatted float.
	#[inline]
	pub fn insert_formatted_float<S: ToOwned<String> + ?Sized>(&mut self, label: &S, item: f64, precision: SignificantDigits, exponent: ExponentFormat) {
		self.content.insert(label.to_owned(), Some(ContentType::FormattedFloat(item, precision, exponent)));
	}

	///Insert a content generator.
	#[inline]
	pub fn insert_generator<S: ToOwned<String> + ?Sized, T: Generator + Send>(&mut self, label: &S, gen: T) {
		self.generators.insert(label.to_owned(), Some(box gen as Box<Generator>));
	}

	///Unset a content generator.
	#[inline]
	pub fn unset_generator<S: ToOwned<String> + ?Sized>(&mut self, label: &S) {
		self.generators.insert(label.to_owned(), None);
	}

	///Set a condition.
	#[inline]
	pub fn set<S: ToOwned<String> + ?Sized>(&mut self, label: &S, value: bool) {
		self.conditions.insert(label.to_owned(), value);
	}

	///Create an other `Shell` around this `Shell`.
	#[inline]
	pub fn wrap<'a: 'c>(&'a self) -> Shell<'a, 'c> {
		Shell::new(self)
	}
}

impl<'r, 'c: 'r> InnerTemplate<'r> for Shell<'r, 'c> {
	fn find_content<'a>(&'a self, label: &String) -> Option<&'a ContentType<'r>> {
		match self.content.find(label) {
			Some(&Some(ref v)) => Some(v),
			Some(&None) => None,
			None => self.base.find_content(label).map(|v| &*v)
		}
	}

	fn get_condition(&self, label: &String) -> bool {
		self.conditions.find(label).map(|&v| v).unwrap_or_else(|| self.base.get_condition(label))
	}
	
	fn is_content_definded(&self, label: &String) -> bool {
		match self.content.find(label) {
			Some(&Some(_)) => true,
			Some(&None) => false,
			None => self.base.is_content_definded(label)
		}
	}

	fn find_generator<'a>(&'a self, label: &String) -> Option<&'a Box<Generator + 'r>> {
		match self.generators.find(label) {
			Some(&Some(ref v)) => Some(v),
			Some(&None) => None,
			None => self.base.find_generator(label).map(|v| &*v)
		}
	}

	fn get_tokens<'a>(&'a self) -> &'a Vec<Token> {
		self.base.get_tokens()
	}
}

impl<'r, 'c> fmt::Show for Shell<'r, 'c> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		format_tokens(self as &InnerTemplate, self.get_tokens(), f)
	}
}




///A trait for overridable templates.
pub trait InnerTemplate<'c> {
	fn find_content<'a>(&'a self, label: &String) -> Option<&'a ContentType<'c>>;
	fn get_condition(&self, label: &String) -> bool;
	fn is_content_definded(&self, label: &String) -> bool;
	fn find_generator<'a>(&'a self, label: &String) -> Option<&'a Box<Generator + 'c>>;
	fn get_tokens<'a>(&'a self) -> &'a Vec<Token>;
}


///A trait for content generators.
pub trait Generator {
	fn generate(&self, args: &Vec<String>, formatter:  &mut fmt::Formatter) -> fmt::Result;
}

impl Generator for fn(args: &Vec<String>, &mut fmt::Formatter) -> fmt::Result {
	fn generate(&self, args: &Vec<String>, formatter:  &mut fmt::Formatter) -> fmt::Result {
		(*self)(args, formatter)
	}
}




fn format_tokens(template: &InnerTemplate, tokens: &Vec<Token>, f: &mut fmt::Formatter) -> fmt::Result {
	for token in tokens.iter() {
		let res = match token {
			&Token::String(ref s) => f.write_str(s.as_slice()),

			&Token::Placeholder(ref k) => {
				match template.find_content(k) {
					Some(value) => value.fmt(f),
					None => Ok(())
				}
			},

			&Token::Conditional(ref k, expected, ref tokens) => {
				if template.get_condition(k) == expected {
					format_tokens(template, tokens, f)
				} else {
					Ok(())
				}
			},

			&Token::ContentConditional(ref k, expected, ref tokens) => {
				if template.is_content_definded(k) == expected {
					format_tokens(template, tokens, f)
				} else {
					Ok(())
				}
			},

			&Token::Generated(ref k, ref vars) => {
				match template.find_generator(k) {
					Some(gen) => gen.generate(vars, f),
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




#[cfg(test)]
mod test {
	use super::{Template, Token, SignificantDigits, ExponentFormat};
	use std::fmt::{Show, Formatter};
	use std::fmt;
	use std::str::from_str;

	macro_rules! test_insert {
		($($v:expr),+) => {
			#[test]
			fn test_insert() {
				let mut template: Template = monitored_from_str("[[:v]]");
				$(
					template.insert("v", $v);
					assert_eq!(template.to_string(), $v.to_string());
				)+
			}
		}
	}

	static peter: &'static str = "Peter";
	static nice: &'static str = "nice";

	fn monitored_from_str(s: &str) -> Template {
		match Template::from_chars(&mut s.chars()) {
			Ok(template) => template,
			Err(e) => panic!(e)
		}
	}

	fn echo(parts: &Vec<String>, f: &mut Formatter) -> fmt::Result {
		parts.connect(":").fmt(f)
	}

	fn echo2(parts: &Vec<String>, f: &mut Formatter) -> fmt::Result {
		parts.connect("_").fmt(f)
	}

	#[test]
	fn basic_tokens() {
		let template: Template = from_str("Hello, [[:name]]! This is a [[:something]] template.").unwrap();
		assert_eq!(template.tokens[0], Token::String("Hello, ".into_string()));
		assert_eq!(template.tokens[1], Token::Placeholder("name".into_string()));
		assert_eq!(template.tokens[2], Token::String("! This is a ".into_string()));
		assert_eq!(template.tokens[3], Token::Placeholder("something".into_string()));
		assert_eq!(template.tokens[4], Token::String(" template.".into_string()));
	}

	#[test]
	#[should_fail]
	fn strange_tokens() {
		let _: Template = from_str("Hello, [[[:name]]]! This is a [[[[:something]] template.").unwrap();
	}

	#[test]
	fn escaped_tokens() {
		let template = monitored_from_str("Hello, [[:name]]! Write placeholders like \\[[:this]] and escape them like \\\\\\[[:this]]");
		assert_eq!(template.tokens[0], Token::String("Hello, ".into_string()));
		assert_eq!(template.tokens[1], Token::Placeholder("name".into_string()));
		assert_eq!(template.tokens[2], Token::String("! Write placeholders like [[:this]] and escape them like \\[[:this]]".into_string()));
	}

	#[test]
	fn replacement() {
		let mut template = monitored_from_str("Hello, [[:name]]! This is a [[:something]] template.");
		template.insert("name", peter);
		template.insert("something", nice);
		assert_eq!(template.to_string(), "Hello, Peter! This is a nice template.".into_string());
	}

	#[test]
	fn templates_in_templates() {
		let mut template1 = monitored_from_str("Hello, [[:name]]! This is a [[:something]] template.");
		let mut template2 = monitored_from_str("really [[:something]]");
		template1.insert("name", peter);
		template2.insert("something", nice);

		template1.insert("something", template2);

		assert_eq!(template1.to_string(), "Hello, Peter! This is a really nice template.".into_string());
	}

	#[test]
	fn conditional() {
		let mut template = monitored_from_str("Hello, [[:name]]![[?condition]] The condition is true.[[/condition]]");
		template.insert("name", peter);
		assert_eq!(template.to_string(), "Hello, Peter!".into_string());
		template.set("condition", true);
		assert_eq!(template.to_string(), "Hello, Peter! The condition is true.".into_string());
	}

	#[test]
	fn conditional_switch() {
		let mut template = monitored_from_str("Hello, [[:name]]! The condition is [[?condition]]true[[/condition]][[?!condition]]false[[/condition]].");
		template.insert("name", peter);
		assert_eq!(template.to_string(), "Hello, Peter! The condition is false.".into_string());
		template.set("condition", true);
		assert_eq!(template.to_string(), "Hello, Peter! The condition is true.".into_string());
	}

	#[test]
	fn content_conditional() {
		let mut template = monitored_from_str("Hello[[?:name]], [[:name]][[/name]]![[?!:name]] I don't know you.[[/!name]]");
		assert_eq!(template.to_string(), "Hello! I don't know you.".into_string());
		template.insert("name", peter);
		assert_eq!(template.to_string(), "Hello, Peter!".into_string());
	}

	#[test]
	fn generator() {
		let mut template = monitored_from_str("[[+\"say hello\" hello Peter    \"how are\" you?]]");
		template.insert_generator("say hello", echo as fn(&Vec<String>, f: &mut Formatter) -> fmt::Result);

		assert_eq!(template.to_string(), "hello:Peter:how are:you?".into_string());
	}

	#[test]
	fn format_float() {
		let mut template = monitored_from_str("[[:short]], [[:long]], [[:default]]");
		template.insert_formatted_float("short", 1.2, SignificantDigits::DigExact(1), ExponentFormat::ExpNone);
		template.insert_formatted_float("long", 1.2, SignificantDigits::DigExact(4), ExponentFormat::ExpNone);
		template.insert("default", 1.2f32);
		assert_eq!(template.to_string(), "1.2, 1.2000, 1.2".into_string())
	}

	test_insert!(1u8, 1u16, 1u32, 1u64, 1i8, 1i16, 1i32, 1i64, 'A', true, false);

	#[test]
	fn wrap_identical() {
		let mut template = monitored_from_str("Hello, [[:name]]! This is a [[:something]] template.");
		template.insert("name", peter);
		template.insert("something", nice);
		let shell = template.wrap();
		assert_eq!(template.to_string(), shell.to_string());
	}

	#[test]
	fn wrap_set() {
		let mut template = monitored_from_str("Hello, [[:name]]! This is a [[:something]] template.");
		template.insert("name", peter);
		template.insert("something", nice);
		let mut shell = template.wrap();
		shell.insert("name", "Olivia");
		assert_eq!(shell.to_string(), "Hello, Olivia! This is a nice template.".into_string());
	}

	#[test]
	fn wrap_unset() {
		let mut template = monitored_from_str("Hello, [[:name]]! This is a [[:something]] template.");
		template.insert("name", peter);
		template.insert("something", nice);
		let mut shell = template.wrap();
		shell.unset("name");
		assert_eq!(shell.to_string(), "Hello, ! This is a nice template.".into_string());
	}

	#[test]
	fn wrap_condition() {
		let mut template = monitored_from_str("Hello, [[:name]]![[?condition]] The condition is true.[[/condition]]");
		template.insert("name", peter);
		template.set("condition", true);
		let mut shell = template.wrap();
		shell.set("condition", false);
		assert_eq!(shell.to_string(), "Hello, Peter!".into_string());
	}

	#[test]
	fn wrap_set_content_condition() {
		let template = monitored_from_str("Hello[[?:name]], [[:name]][[/name]]![[?!:name]] I don't know you.[[/!name]]");
		let mut shell = template.wrap();
		shell.insert("name", peter);
		assert_eq!(shell.to_string(), "Hello, Peter!".into_string());
	}

	#[test]
	fn wrap_unset_content_condition() {
		let mut template = monitored_from_str("Hello[[?:name]], [[:name]][[/name]]![[?!:name]] I don't know you.[[/!name]]");
		template.insert("name", peter);
		let mut shell = template.wrap();
		shell.unset("name");
		assert_eq!(shell.to_string(), "Hello! I don't know you.".into_string());
	}

	#[test]
	fn wrap_set_generator() {
		let mut template = monitored_from_str("[[+\"say hello\" hello Peter    \"how are\" you?]]");
		template.insert_generator("say hello", echo as fn(&Vec<String>, f: &mut Formatter) -> fmt::Result);
		let mut shell = template.wrap();
		shell.insert_generator("say hello", echo2 as fn(&Vec<String>, f: &mut Formatter) -> fmt::Result);

		assert_eq!(shell.to_string(), "hello_Peter_how are_you?".into_string());
	}

	#[test]
	fn wrap_unset_generator() {
		let mut template = monitored_from_str("[[+\"say hello\" hello Peter    \"how are\" you?]]");
		template.insert_generator("say hello", echo as fn(&Vec<String>, f: &mut Formatter) -> fmt::Result);
		let mut shell = template.wrap();
		shell.unset_generator("say hello");

		assert_eq!(shell.to_string(), "".into_string());
	}
}