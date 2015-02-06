use super::Token;

use std::iter::{Iterator, Peekable};
use std::fmt;

#[derive(PartialEq)]
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
	fn push_to_buf(&self, buf: &mut String) {
		match *self {
			LexToken::Begin => {
				buf.push('[');
				buf.push('[');
			},
			LexToken::End => {
				buf.push(']');
				buf.push(']');
			},
			LexToken::Colon => buf.push(':'),
			LexToken::Questionmark => buf.push('?'),
			LexToken::Exclamation => buf.push('!'),
			LexToken::Plus => buf.push('+'),
			LexToken::Slash => buf.push('/'),
			LexToken::Quote => buf.push('"'),
			LexToken::Character(c) => buf.push(c)
		}
	}
}

impl fmt::Display for LexToken {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			LexToken::Begin => {
				'['.fmt(f).and_then(|_| '['.fmt(f))
			},
			LexToken::End => ']'.fmt(f).and_then(|_| ']'.fmt(f)),
			LexToken::Colon => ':'.fmt(f),
			LexToken::Questionmark => '?'.fmt(f),
			LexToken::Exclamation => '!'.fmt(f),
			LexToken::Plus => '+'.fmt(f),
			LexToken::Slash => '/'.fmt(f),
			LexToken::Quote => '"'.fmt(f),
			LexToken::Character(c) => c.fmt(f)
		}
	}
}


struct Parser<I: Iterator> {
    tokens: Peekable<I>
}

impl<I: Iterator<Item=V>, V: PartialEq> Parser<I> {
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

	fn eat_while<F: Fn(&V) -> bool>(&mut self, is_edible: F) {
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

impl<I: Iterator<Item=V>, V> Iterator for Parser<I> {
	type Item = V;

	#[inline]
	fn next(&mut self) -> Option<V> {
		self.tokens.by_ref().next()
	}
}

pub fn parse<T: Iterator<Item=Result<char, String>>>(chars: T) -> Result<Vec<Token>, String> {
	let tokens = try!(lex(chars));
	parse_block(&mut Parser{
		tokens: tokens.into_iter().by_ref().peekable()
	})
}

fn lex<T: Iterator<Item=Result<char, String>>>(chars: T) -> Result<Vec<LexToken>, String> {
	let mut chars = chars.peekable();
	let mut tokens = Vec::new();

	loop {
		match chars.next() {
			Some(Ok(c)) => match c {
				'[' => match chars.peek() {
					Some(&Ok('[')) => {
						tokens.push(LexToken::Begin);
						chars.next();
					},
					_ => tokens.push(LexToken::Character('['))
				},
				']' => match chars.peek() {
					Some(&Ok(']')) => {
						tokens.push(LexToken::End);
						chars.next();
					},
					_ => tokens.push(LexToken::Character(']'))
				},
				':' => tokens.push(LexToken::Colon),
				'?' => tokens.push(LexToken::Questionmark),
				'!' => tokens.push(LexToken::Exclamation),
				'+' => tokens.push(LexToken::Plus),
				'/' => tokens.push(LexToken::Slash),
				'"' => tokens.push(LexToken::Quote),
				'\\' => match chars.next() {
					Some(Ok(c)) => {
						tokens.push(LexToken::Character(c));
					},
					Some(Err(e)) => return Err(e),
					None => break
				},
				c => tokens.push(LexToken::Character(c))
			},
			Some(Err(e)) => return Err(e),
			None => break
		}
	}

	Ok(tokens)
}

fn parse_block<I: Iterator<Item=LexToken>>(tokens: &mut Parser<I>) -> Result<Vec<Token>, String> {
	let mut result = Vec::new();
	let mut string = String::new();

	loop {
		match tokens.next() {
			Some(LexToken::Begin) => match tokens.next() {
				Some(LexToken::Colon) => {
					if string.len() > 0 {
						result.push(Token::String(string));
						string = String::new();
					}

					result.push(try!(parse_placeholder(tokens)));
				},
				Some(LexToken::Questionmark) => {
					if string.len() > 0 {
						result.push(Token::String(string));
						string = String::new();
					}

					result.push(try!(parse_conditional(tokens)));
				},
				Some(LexToken::Plus) => {
					if string.len() > 0 {
						result.push(Token::String(string));
						string = String::new();
					}

					result.push(try!(parse_generator(tokens)));
				},
				Some(LexToken::Slash) => {
					if string.len() > 0 {
						result.push(Token::String(string));
						string = String::new();
					}

					parse_block_end(tokens);
					break
				},
				Some(t) => {
					return Err(format!("parse error: unknown token type: '{}'", t))
				},
				None => LexToken::Begin.push_to_buf(&mut string)
			},
			Some(t) => t.push_to_buf(&mut string),
			None => break
		}
	}

	if string.len() > 0 {
		result.push(Token::String(string));
	}

	Ok(result)
}

fn parse_placeholder<I: Iterator<Item=LexToken>>(tokens: &mut Parser<I>) -> Result<Token, String> {
	let mut label = String::new();

	for t in tokens.by_ref().take_while(|t| *t != LexToken::End) {
		t.push_to_buf(&mut label);
	}

	Ok(Token::Placeholder(label))
}

fn parse_conditional<I: Iterator<Item=LexToken>>(tokens: &mut Parser<I>) -> Result<Token, String> {
	let negative = tokens.eat(LexToken::Exclamation);
	let content_cond = tokens.eat(LexToken::Colon);
	let mut label = String::new();

	for t in tokens.by_ref().take_while(|t| *t != LexToken::End) {
		t.push_to_buf(&mut label);
	}

	let content = try!(parse_block(tokens));

	if content_cond {
		Ok(Token::ContentConditional(label, !negative, content))
	} else {
		Ok(Token::Conditional(label, !negative, content))
	}
}

fn parse_generator<I: Iterator<Item=LexToken>>(tokens: &mut Parser<I>) -> Result<Token, String> {
	let mut label = String::new();
	let mut args = Vec::new();

	if tokens.eat(LexToken::Quote) {
		for t in tokens.by_ref().take_while(|t| *t != LexToken::Quote) {
			t.push_to_buf(&mut label);
		}
	} else {
		loop {
			match tokens.next() {
				Some(LexToken::End) => return Ok(Token::Generated(label, Vec::new())),
				Some(LexToken::Character(c)) if c.is_whitespace() => break,
				Some(t) => t.push_to_buf(&mut label),
				None => break
			}
		}
	}

	'arg_list: loop {
		let mut new_arg = String::new();

		tokens.eat_while(|t| match *t {LexToken::Character(c) if c.is_whitespace() => true, _ => false});

		if tokens.eat(LexToken::Quote) {
			for t in tokens.by_ref().take_while(|t| *t != LexToken::Quote) {
				t.push_to_buf(&mut new_arg);
			}
		} else {
			'arg: loop {
				match tokens.next() {
					Some(LexToken::End) => if new_arg.len() > 0 {
						args.push(new_arg);
						break 'arg_list;
					},
					Some(LexToken::Character(c)) if c.is_whitespace() => {
						tokens.eat_while(|t| match *t {LexToken::Character(c) if c.is_whitespace() => true, _ => false});
						break 'arg
					},
					Some(t) => t.push_to_buf(&mut new_arg),
					None => break 'arg_list
				}
			}
		}

		args.push(new_arg);
	}
	

	Ok(Token::Generated(label, args))
}

fn parse_block_end<I: Iterator<Item=LexToken>>(tokens: &mut Parser<I>) {
	tokens.by_ref().all(|t| t != LexToken::End);
}

