#![doc(html_root_url = "http://ogeon.github.io/fragments/doc/")]
#![feature(core, std_misc, old_io)]

use std::fmt;
use std::str::FromStr;
use std::vec::Vec;
use std::collections::{HashMap, HashSet};
use std::num::strconv::{
	float_to_str_common,
	SignFormat
};

pub use std::num::strconv::{
	SignificantDigits,
	ExponentFormat
};

mod parser;

///Internal representation of template parts.
#[derive(PartialEq, Debug)]
pub enum Token {
	String(String),
	Placeholder(String),
	Conditional(String, bool, Vec<Token>),
	ContentConditional(String, bool, Vec<Token>),
	Generated(String, Vec<String>)
}

///Container enum for template content
pub enum ContentType<'c> {
	Float(f64),
	FormattedFloat(f64, SignificantDigits, ExponentFormat),
	Int(i64),
	UnsignedInt(u64),
	Char(char),
	Bool(bool),
	String(String),
	StringSlice(&'c str),
	Template(Template<'c>),
	Shell(Shell<'c, 'c>),
	Fmt(Box<fmt::Display + 'c>)
}

macro_rules! call_fmt {
	($slf:ident, $f:ident: $($p:pat => $b:expr),+ and $($t:ident),+) => {
		match $slf {
			$($p => $b,)+
			$(&ContentType::$t(ref v) => fmt::Display::fmt(v, $f)),+
		}
	}
}

impl<'c> fmt::Display for ContentType<'c> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		call_fmt! {
			self,
			f:
			&ContentType::FormattedFloat(v, sig, exp) => {
				let (string, _special) = float_to_str_common(v, 10, false, SignFormat::SignNeg, sig, exp, false);
				fmt::Display::fmt(&string, f)
			},
			&ContentType::Fmt(ref v) => {
				use std::fmt::Display;
				(**v).fmt(f)
			}
			
			and

			Int,
			UnsignedInt,
			Float,
			Char,
			Bool,
			String,
			StringSlice,
			Template,
			Shell
		}
	}
}


///A trait for types that can be inserted into templates
pub trait TemplateContent<'c> {
	///Convert `self` to a suitable `ContentType` variant, not making a copy if possible.
	fn into_template_content(self) -> ContentType<'c>;
}

macro_rules! float_content {
	($($t:ty),+) => {
		$(impl TemplateContent<'static> for $t {
			fn into_template_content(self) -> ContentType<'static> {
				ContentType::Float(self as f64)
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
		})+
	}
}

macro_rules! uint_content {
	($($t:ty),+) => {
		$(impl TemplateContent<'static> for $t {
			fn into_template_content(self) -> ContentType<'static> {
				ContentType::UnsignedInt(self as u64)
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
		})+
	}
}

float_content!(f32, f64);
int_content!(isize, i8, i16, i32, i64);
uint_content!(usize, u8, u16, u32, u64);
deref_content!([char, Char], [bool, Bool]);


impl TemplateContent<'static> for String {
	fn into_template_content(self) -> ContentType<'static> {
		ContentType::String(self)
	}
}


impl<'c> TemplateContent<'c> for &'c str {
	fn into_template_content(self) -> ContentType<'c> {
		ContentType::StringSlice(self)
	}
}


impl<'c> TemplateContent<'c> for Template<'c> {
	fn into_template_content(self) -> ContentType<'c> {
		ContentType::Template(self)
	}
}


impl<'r, 'c: 'r> TemplateContent<'r> for Shell<'r, 'c> {
	fn into_template_content(self) -> ContentType<'r> {
		ContentType::Shell(self)
	}
}


impl<'c> TemplateContent<'c> for Box<fmt::Display + 'c> {
	fn into_template_content(self) -> ContentType<'c> {
		ContentType::Fmt(self)
	}
}


impl<'c> TemplateContent<'c> for ContentType<'c> {
	fn into_template_content(self) -> ContentType<'c> {
		self
	}
}

///A string template with placeholders and conditional content.
///
///Placeholders are written as `[[:label]]`, where `label` becomes the name of the placeholder.
///The label is then used to insert content: `my_template.insert("label", my_content);`.
///The assigned content for a placeholder can be anything that implements `Display`.
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
pub struct Template<'c> {
	///Content for the placeholders
	pub content: HashMap<String, ContentType<'c>>,
	///Content generators
	pub generators: HashMap<String, Box<Generator + 'c>>,
	///Conditional switches
	pub conditions: HashSet<String>,
	tokens: Vec<Token>
}

impl<'c> Template<'c> {
	///Create a new `Template` from a character iterator.
	#[inline]
	pub fn from_chars(b: std::str::Chars) -> Result<Template<'c>, String> {
		let tokens = try!(parser::parse(b.map(|r| Ok::<char, String>(r))));

		Ok(Template {
			content: HashMap::new(),
			generators: HashMap::new(),
			conditions: HashSet::new(),
			tokens: tokens
		})
	}

	///Create a new `Template` from a buffer.
	#[inline]
	pub fn from_buffer<T: Buffer>(b: &mut T) -> Result<Template<'c>, String> {
		let tokens = try!(parser::parse(b.chars().map(|r| match r {
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
	pub fn insert<T: TemplateContent<'c>>(&mut self, label: String, item: T) {
		self.content.insert(label, item.into_template_content());
	}

	///Insert a formatted float.
	#[inline]
	pub fn insert_formatted_float(&mut self, label: String, item: f64, precision: SignificantDigits, exponent: ExponentFormat) {
		self.content.insert(label, ContentType::FormattedFloat(item, precision, exponent));
	}

	///Insert a content generator.
	#[inline]
	pub fn insert_generator<T: Generator + Send + 'c>(&mut self, label: String, gen: T) {
		self.generators.insert(label, Box::new(gen) as Box<Generator>);
	}

	///Set a condition.
	#[inline]
	pub fn set(&mut self, label: String, value: bool) {
		if value {
			self.conditions.insert(label);
		} else {
			self.conditions.remove(&label);
		}
	}

	///Create a `Shell` around this `Template`.
	#[inline]
	pub fn wrap<'a: 'c>(&'a self) -> Shell<'a, 'c> {
		Shell::new(self)
	}

	pub fn get_tokens(&self) -> &[Token] {
		&*self.tokens
	}
}

impl<'c> InnerTemplate<'c> for Template<'c> {
	fn get_content<'a>(&'a self, label: &str) -> Option<&'a ContentType<'c>> {
		self.content.get(label)
	}

	fn get_condition(&self, label: &str) -> bool {
		self.conditions.contains(label)
	}
	
	fn is_content_defined(&self, label: &str) -> bool {
		self.content.contains_key(label)
	}

	fn get_generator<'a>(&'a self, label: &str) -> Option<&'a Generator> {
		self.generators.get(label).map(|v| &**v)
	}

	fn render(&self, top_template: &InnerTemplate, f: &mut fmt::Formatter) -> fmt::Result {
		format_tokens(top_template, self.tokens.as_slice(), f)
	}
}

impl<'c> FromStr for Template<'c> {
	type Err = String;
	
	///Creates a new `Template` from a string.
	fn from_str(s: &str) -> Result<Template<'c>, String> {
		Template::from_chars(s.chars())
	}
}

impl<'c> fmt::Display for Template<'c> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		format_tokens(self as &InnerTemplate, self.tokens.as_slice(), f)
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
	pub fn new<T: InnerTemplate<'c>>(base: &'r T) -> Shell<'r, 'c> {
		Shell {
			content: HashMap::new(),
			generators: HashMap::new(),
			conditions: HashMap::new(),
			base: base as &InnerTemplate<'c>
		}
	}

	///Insert content.
	#[inline]
	pub fn insert<T: TemplateContent<'c>>(&mut self, label: String, item: T) {
		self.content.insert(label, Some(item.into_template_content()));
	}

	///Unset content.
	#[inline]
	pub fn unset(&mut self, label: String) {
		self.content.insert(label, None);
	}

	///Insert a formatted float.
	#[inline]
	pub fn insert_formatted_float(&mut self, label: String, item: f64, precision: SignificantDigits, exponent: ExponentFormat) {
		self.content.insert(label, Some(ContentType::FormattedFloat(item, precision, exponent)));
	}

	///Insert a content generator.
	#[inline]
	pub fn insert_generator<T: Generator + Send + 'c>(&mut self, label: String, gen: T) {
		self.generators.insert(label, Some(Box::new(gen) as Box<Generator>));
	}

	///Unset a content generator.
	#[inline]
	pub fn unset_generator(&mut self, label: String) {
		self.generators.insert(label, None);
	}

	///Set a condition.
	#[inline]
	pub fn set(&mut self, label: String, value: bool) {
		self.conditions.insert(label, value);
	}

	///Create an other `Shell` around this `Shell`.
	#[inline]
	pub fn wrap<'a: 'c>(&'a self) -> Shell<'a, 'c> {
		Shell::new(self)
	}
}

impl<'r, 'c: 'r> InnerTemplate<'r> for Shell<'r, 'c> {
	fn get_content<'a>(&'a self, label: &str) -> Option<&'a ContentType<'r>> {
		match self.content.get(label) {
			Some(&Some(ref v)) => Some(v),
			Some(&None) => None,
			None => self.base.get_content(label).map(|v| &*v)
		}
	}

	fn get_condition(&self, label: &str) -> bool {
		self.conditions.get(label).map(|&v| v).unwrap_or_else(|| self.base.get_condition(label))
	}
	
	fn is_content_defined(&self, label: &str) -> bool {
		match self.content.get(label) {
			Some(&Some(_)) => true,
			Some(&None) => false,
			None => self.base.is_content_defined(label)
		}
	}

	fn get_generator<'a>(&'a self, label: &str) -> Option<&'a Generator> {
		match self.generators.get(label) {
			Some(&Some(ref v)) => Some(&**v),
			Some(&None) => None,
			None => self.base.get_generator(label).map(|v| &*v)
		}
	}

	fn render(&self, top_template: &InnerTemplate, f: &mut fmt::Formatter) -> fmt::Result {
		self.base.render(top_template, f)
	}
}

impl<'r, 'c> fmt::Display for Shell<'r, 'c> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.base.render(self as &InnerTemplate, f)
	}
}




///A trait for overridable templates.
pub trait InnerTemplate<'c> {
	fn get_content<'a>(&'a self, label: &str) -> Option<&'a ContentType<'c>>;
	fn get_condition(&self, label: &str) -> bool;
	fn is_content_defined(&self, label: &str) -> bool;
	fn get_generator<'a>(&'a self, label: &str) -> Option<&'a Generator>;
	fn render(&self, top_template: &InnerTemplate, f: &mut fmt::Formatter) -> fmt::Result;
}


///A trait for content generators.
pub trait Generator {
	fn generate(&self, args: &[String], formatter:  &mut fmt::Formatter) -> fmt::Result;
}

impl<F: Fn(&[String], & mut fmt::Formatter) -> fmt::Result> Generator for F {
	fn generate(&self, args: &[String], formatter:  &mut fmt::Formatter) -> fmt::Result {
		(*self)(args, formatter)
	}
}




fn format_tokens(template: &InnerTemplate, tokens: &[Token], f: &mut fmt::Formatter) -> fmt::Result {
	for token in tokens.iter() {
		let res = match token {
			&Token::String(ref s) => f.write_str(s.as_slice()),

			&Token::Placeholder(ref k) => {
				match template.get_content(k.as_slice()) {
					Some(value) => fmt::Display::fmt(value, f),
					None => Ok(())
				}
			},

			&Token::Conditional(ref k, expected, ref tokens) => {
				if template.get_condition(k.as_slice()) == expected {
					format_tokens(template, tokens.as_slice(), f)
				} else {
					Ok(())
				}
			},

			&Token::ContentConditional(ref k, expected, ref tokens) => {
				if template.is_content_defined(k.as_slice()) == expected {
					format_tokens(template, tokens.as_slice(), f)
				} else {
					Ok(())
				}
			},

			&Token::Generated(ref k, ref vars) => {
				match template.get_generator(k.as_slice()) {
					Some(gen) => gen.generate(vars.as_slice(), f),
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
	use std::fmt;
	use std::borrow::ToOwned;

	macro_rules! test_insert {
		($($v:expr),+) => {
			#[test]
			fn test_insert() {
				let mut template: Template = monitored_from_str("[[:v]]");
				$(
					template.insert("v".to_owned(), $v);
					assert_eq!(template.to_string(), $v.to_string());
				)+
			}
		}
	}

	static PETER: &'static str = "Peter";
	static NICE: &'static str = "nice";

	fn monitored_from_str(s: &str) -> Template {
		match Template::from_chars(s.chars()) {
			Ok(template) => template,
			Err(e) => panic!(e)
		}
	}

	fn echo(parts: &[String], f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Display::fmt(&parts.connect(":"), f)
	}

	fn echo2(parts: &[String], f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Display::fmt(&parts.connect("_"), f)
	}

	#[test]
	fn basic_tokens() {
		let template: Template = "Hello, [[:name]]! This is a [[:something]] template.".parse().unwrap();
		assert_eq!(template.tokens[0], Token::String("Hello, ".to_owned()));
		assert_eq!(template.tokens[1], Token::Placeholder("name".to_owned()));
		assert_eq!(template.tokens[2], Token::String("! This is a ".to_owned()));
		assert_eq!(template.tokens[3], Token::Placeholder("something".to_owned()));
		assert_eq!(template.tokens[4], Token::String(" template.".to_owned()));
	}

	#[test]
	#[should_fail]
	fn strange_tokens() {
		let _: Template = "Hello, [[[:name]]]! This is a [[[[:something]] template.".parse().unwrap();
	}

	#[test]
	fn escaped_tokens() {
		let template = monitored_from_str("Hello, [[:name]]! Write placeholders like \\[[:this]] and escape them like \\\\\\[[:this]]");
		assert_eq!(template.tokens[0], Token::String("Hello, ".to_owned()));
		assert_eq!(template.tokens[1], Token::Placeholder("name".to_owned()));
		assert_eq!(template.tokens[2], Token::String("! Write placeholders like [[:this]] and escape them like \\[[:this]]".to_owned()));
	}

	#[test]
	fn replacement() {
		let mut template = monitored_from_str("Hello, [[:name]]! This is a [[:something]] template.");
		template.insert("name".to_owned(), PETER);
		template.insert("something".to_owned(), NICE);
		assert_eq!(template.to_string(), "Hello, Peter! This is a nice template.".to_owned());
	}

	#[test]
	fn templates_in_templates() {
		let mut template1 = monitored_from_str("Hello, [[:name]]! This is a [[:something]] template.");
		let mut template2 = monitored_from_str("really [[:something]]");
		template1.insert("name".to_owned(), PETER);
		template2.insert("something".to_owned(), NICE);

		template1.insert("something".to_owned(), template2);

		assert_eq!(template1.to_string(), "Hello, Peter! This is a really nice template.".to_owned());
	}

	#[test]
	fn conditional() {
		let mut template = monitored_from_str("Hello, [[:name]]![[?condition]] The condition is true.[[/condition]]");
		template.insert("name".to_owned(), PETER);
		assert_eq!(template.to_string(), "Hello, Peter!".to_owned());
		template.set("condition".to_owned(), true);
		assert_eq!(template.to_string(), "Hello, Peter! The condition is true.".to_owned());
	}

	#[test]
	fn conditional_switch() {
		let mut template = monitored_from_str("Hello, [[:name]]! The condition is [[?condition]]true[[/condition]][[?!condition]]false[[/condition]].");
		template.insert("name".to_owned(), PETER);
		assert_eq!(template.to_string(), "Hello, Peter! The condition is false.".to_owned());
		template.set("condition".to_owned(), true);
		assert_eq!(template.to_string(), "Hello, Peter! The condition is true.".to_owned());
	}

	#[test]
	fn content_conditional() {
		let mut template = monitored_from_str("Hello[[?:name]], [[:name]][[/name]]![[?!:name]] I don't know you.[[/!name]]");
		assert_eq!(template.to_string(), "Hello! I don't know you.".to_owned());
		template.insert("name".to_owned(), PETER);
		assert_eq!(template.to_string(), "Hello, Peter!".to_owned());
	}

	#[test]
	fn generator() {
		let mut template = monitored_from_str("[[+\"say hello\" hello Peter    \"how are\" you?]]");

		template.insert_generator("say hello".to_owned(), echo);
		assert_eq!(template.to_string(), "hello:Peter:how are:you?".to_owned());

		template.insert_generator("say hello".to_owned(), |parts: &[String], f: &mut fmt::Formatter| fmt::Display::fmt(&parts.connect(":"), f));
		assert_eq!(template.to_string(), "hello:Peter:how are:you?".to_owned());
	}

	#[test]
	fn format_float() {
		let mut template = monitored_from_str("[[:short]], [[:long]], [[:default]]");
		template.insert_formatted_float("short".to_owned(), 1.2, SignificantDigits::DigExact(1), ExponentFormat::ExpNone);
		template.insert_formatted_float("long".to_owned(), 1.2, SignificantDigits::DigExact(4), ExponentFormat::ExpNone);
		template.insert("default".to_owned(), 1.2f32);
		assert_eq!(template.to_string(), "1.2, 1.2000, 1.2".to_owned())
	}

	test_insert!(1u8, 1u16, 1u32, 1u64, 1i8, 1i16, 1i32, 1i64, 'A', true, false);

	#[test]
	fn wrap_identical() {
		let mut template = monitored_from_str("Hello, [[:name]]! This is a [[:something]] template.");
		template.insert("name".to_owned(), PETER);
		template.insert("something".to_owned(), NICE);
		let shell = template.wrap();
		assert_eq!(template.to_string(), shell.to_string());
	}

	#[test]
	fn wrap_set() {
		let mut template = monitored_from_str("Hello, [[:name]]! This is a [[:something]] template.");
		template.insert("name".to_owned(), PETER);
		template.insert("something".to_owned(), NICE);
		let mut shell = template.wrap();
		shell.insert("name".to_owned(), "Olivia");
		assert_eq!(shell.to_string(), "Hello, Olivia! This is a nice template.".to_owned());
	}

	#[test]
	fn wrap_unset() {
		let mut template = monitored_from_str("Hello, [[:name]]! This is a [[:something]] template.");
		template.insert("name".to_owned(), PETER);
		template.insert("something".to_owned(), NICE);
		let mut shell = template.wrap();
		shell.unset("name".to_owned());
		assert_eq!(shell.to_string(), "Hello, ! This is a nice template.".to_owned());
	}

	#[test]
	fn wrap_condition() {
		let mut template = monitored_from_str("Hello, [[:name]]![[?condition]] The condition is true.[[/condition]]");
		template.insert("name".to_owned(), PETER);
		template.set("condition".to_owned(), true);
		let mut shell = template.wrap();
		shell.set("condition".to_owned(), false);
		assert_eq!(shell.to_string(), "Hello, Peter!".to_owned());
	}

	#[test]
	fn wrap_set_content_condition() {
		let template = monitored_from_str("Hello[[?:name]], [[:name]][[/name]]![[?!:name]] I don't know you.[[/!name]]");
		let mut shell = template.wrap();
		shell.insert("name".to_owned(), PETER);
		assert_eq!(shell.to_string(), "Hello, Peter!".to_owned());
	}

	#[test]
	fn wrap_unset_content_condition() {
		let mut template = monitored_from_str("Hello[[?:name]], [[:name]][[/name]]![[?!:name]] I don't know you.[[/!name]]");
		template.insert("name".to_owned(), PETER);
		let mut shell = template.wrap();
		shell.unset("name".to_owned());
		assert_eq!(shell.to_string(), "Hello! I don't know you.".to_owned());
	}

	#[test]
	fn wrap_set_generator() {
		let mut template = monitored_from_str("[[+\"say hello\" hello Peter    \"how are\" you?]]");
		template.insert_generator("say hello".to_owned(), echo);
		let mut shell = template.wrap();
		shell.insert_generator("say hello".to_owned(), echo2);

		assert_eq!(shell.to_string(), "hello_Peter_how are_you?".to_owned());
	}

	#[test]
	fn wrap_unset_generator() {
		let mut template = monitored_from_str("[[+\"say hello\" hello Peter    \"how are\" you?]]");
		template.insert_generator("say hello".to_owned(), echo);
		let mut shell = template.wrap();
		shell.unset_generator("say hello".to_owned());

		assert_eq!(shell.to_string(), "".to_owned());
	}

	#[test]
	fn shells_in_templates() {
		let template2 = monitored_from_str("really [[:something]]");
		let mut template1 = monitored_from_str("Hello, [[:name]]! This is a [[:something]] template.");
		let mut shell = template2.wrap();
		template1.insert("name".to_owned(), PETER);
		shell.insert("something".to_owned(), NICE);

		template1.insert("something".to_owned(), shell);

		assert_eq!(template1.to_string(), "Hello, Peter! This is a really nice template.".to_owned());
	}
}