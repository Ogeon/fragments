use super::{Token, String, Placeholder, Conditional, ContentConditional, Generated};

use std::iter::{Iterator, Peekable};
use std::fmt;

#[deriving(Eq)]
enum LexToken {
	Begin,
	End,
	Colon,
	Questionmark,
	Exclamation,
	Plus,
	Slash,
	Quote,
	Character(char)
}

impl LexToken {
	fn push_to_buf(&self, buf: &mut StrBuf) {
		match *self {
			Begin => {
				buf.push_char('[');
				buf.push_char('[');
			},
			End => {
				buf.push_char(']');
				buf.push_char(']');
			},
			Colon => buf.push_char(':'),
			Questionmark => buf.push_char('?'),
			Exclamation => buf.push_char('!'),
			Plus => buf.push_char('+'),
			Slash => buf.push_char('/'),
			Quote => buf.push_char('"'),
			Character(c) => buf.push_char(c)
		}
	}
}

impl fmt::Show for LexToken {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			Begin => {
				'['.fmt(f).and_then(|_| '['.fmt(f))
			},
			End => ']'.fmt(f).and_then(|_| ']'.fmt(f)),
			Colon => ':'.fmt(f),
			Questionmark => '?'.fmt(f),
			Exclamation => '!'.fmt(f),
			Plus => '+'.fmt(f),
			Slash => '/'.fmt(f),
			Quote => '"'.fmt(f),
			Character(c) => c.fmt(f)
		}
	}
}


struct Parser<T, V> {
    tokens: Peekable<V, T>
}

impl<T: Iterator<V>, V: Eq> Parser<T, V> {
	#[inline]
	fn eat(&mut self, expected: V) -> bool {
		let eaten = match self.peek() {
			Some(t) if *t == expected => {
				true
			},
			_ => false
		};

		if eaten {
			self.next();
		}

		eaten
	}

	fn eat_while(&mut self, is_edible: |&V| -> bool) {
		loop {
			let eaten = match self.peek() {
				Some(t) => is_edible(t),
				_ => false
			};

			if eaten {
				self.next();
			} else {
				break;
			}
		}
	}

	#[inline]
	fn peek<'a>(&'a mut self) -> Option<&'a V> {
		self.tokens.peek()
	}
}

impl<T: Iterator<V>, V> Iterator<V> for Parser<T, V> {
	#[inline]
	fn next(&mut self) -> Option<V> {
		self.tokens.by_ref().next()
	}
}

pub fn parse<T: Iterator<Result<char, StrBuf>>>(chars: &mut T) -> Result<Vec<Token>, StrBuf> {
	let tokens = try!(lex(chars));
	parse_block(&mut Parser{
		tokens: tokens.move_iter().by_ref().peekable()
	})
}

fn lex<T: Iterator<Result<char, StrBuf>>>(chars: &mut T) -> Result<Vec<LexToken>, StrBuf> {
	let mut chars = chars.by_ref().peekable();
	let mut tokens = Vec::new();

	loop {
		match chars.next() {
			Some(Ok(c)) => match c {
				'[' => match chars.peek() {
					Some(&Ok('[')) => {
						tokens.push(Begin);
						chars.next();
					},
					_ => tokens.push(Character('['))
				},
				']' => match chars.peek() {
					Some(&Ok(']')) => {
						tokens.push(End);
						chars.next();
					},
					_ => tokens.push(Character(']'))
				},
				':' => tokens.push(Colon),
				'?' => tokens.push(Questionmark),
				'!' => tokens.push(Exclamation),
				'+' => tokens.push(Plus),
				'/' => tokens.push(Slash),
				'"' => tokens.push(Quote),
				'\\' => match chars.next() {
					Some(Ok(c)) => {
						tokens.push(Character(c));
					},
					Some(Err(e)) => return Err(e.into_strbuf()),
					None => break
				},
				c => tokens.push(Character(c))
			},
			Some(Err(e)) => return Err(e),
			None => break
		}
	}

	Ok(tokens)
}

fn parse_block<T: Iterator<LexToken>>(tokens: &mut Parser<T, LexToken>) -> Result<Vec<Token>, StrBuf> {
	let mut result = Vec::new();
	let mut string = StrBuf::new();

	loop {
		match tokens.next() {
			Some(Begin) => match tokens.next() {
				Some(Colon) => {
					if string.len() > 0 {
						result.push(String(string));
						string = StrBuf::new();
					}

					result.push(try!(parse_placeholder(tokens)));
				},
				Some(Questionmark) => {
					if string.len() > 0 {
						result.push(String(string));
						string = StrBuf::new();
					}

					result.push(try!(parse_conditional(tokens)));
				},
				Some(Plus) => {
					if string.len() > 0 {
						result.push(String(string));
						string = StrBuf::new();
					}

					result.push(try!(parse_generator(tokens)));
				},
				Some(Slash) => {
					if string.len() > 0 {
						result.push(String(string));
						string = StrBuf::new();
					}

					parse_block_end(tokens);
					break
				},
				Some(t) => {
					return Err(format_strbuf!("parse error: unknown token type: '{}'", t))
				},
				None => Begin.push_to_buf(&mut string)
			},
			Some(t) => t.push_to_buf(&mut string),
			None => break
		}
	}

	if string.len() > 0 {
		result.push(String(string));
	}

	Ok(result)
}

fn parse_placeholder<T: Iterator<LexToken>>(tokens: &mut Parser<T, LexToken>) -> Result<Token, StrBuf> {
	let mut label = StrBuf::new();

	for t in tokens.by_ref().take_while(|&t| t != End) {
		t.push_to_buf(&mut label);
	}

	Ok(Placeholder(label))
}

fn parse_conditional<T: Iterator<LexToken>>(tokens: &mut Parser<T, LexToken>) -> Result<Token, StrBuf> {
	let negative = tokens.eat(Exclamation);
	let content_cond = tokens.eat(Colon);
	let mut label = StrBuf::new();

	for t in tokens.by_ref().take_while(|&t| t != End) {
		t.push_to_buf(&mut label);
	}

	let content = try!(parse_block(tokens));

	if content_cond {
		Ok(ContentConditional(label, !negative, content))
	} else {
		Ok(Conditional(label, !negative, content))
	}
}

fn parse_generator<T: Iterator<LexToken>>(tokens: &mut Parser<T, LexToken>) -> Result<Token, StrBuf> {
	let mut label = StrBuf::new();
	let mut args = Vec::new();

	if tokens.eat(Quote) {
		for t in tokens.by_ref().take_while(|&t| t != Quote) {
			t.push_to_buf(&mut label);
		}
	} else {
		loop {
			match tokens.next() {
				Some(End) => return Ok(Generated(label, Vec::new())),
				Some(Character(c)) if c.is_whitespace() => break,
				Some(t) => t.push_to_buf(&mut label),
				None => break
			}
		}
	}

	'arg_list: loop {
		let mut new_arg = StrBuf::new();

		tokens.eat_while(|&t| match t {Character(c) if c.is_whitespace() => true, _ => false});

		if tokens.eat(Quote) {
			for t in tokens.by_ref().take_while(|&t| t != Quote) {
				t.push_to_buf(&mut new_arg);
			}
		} else {
			'arg: loop {
				match tokens.next() {
					Some(End) => if new_arg.len() > 0 {
						args.push(new_arg);
						break 'arg_list;
					},
					Some(Character(c)) if c.is_whitespace() => {
						tokens.eat_while(|&t| match t {Character(c) if c.is_whitespace() => true, _ => false});
						break 'arg
					},
					Some(t) => t.push_to_buf(&mut new_arg),
					None => break 'arg_list
				}
			}
		}

		args.push(new_arg);
	}
	

	Ok(Generated(label, args))
}

fn parse_block_end<T: Iterator<LexToken>>(tokens: &mut Parser<T, LexToken>) {
	tokens.by_ref().advance(|t| t != End);
}

