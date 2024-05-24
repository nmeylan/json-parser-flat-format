use crate::string_from_bytes;

#[derive(Debug)]
pub enum Token<'a> {
    CurlyOpen,
    CurlyClose,
    SquareOpen,
    SquareClose,
    Colon,
    Comma,
    String(&'a str),
    Number(&'a str),
    Boolean(bool),
    Null,
}


pub struct SliceRead<'a> {
    slice: &'a [u8],
    index: usize,
}

impl<'a> SliceRead<'a> {
    pub fn new(slice: &'a [u8]) -> Self {
        SliceRead { slice, index: 0 }
    }
    #[inline]
    pub fn next(&mut self) -> Option<u8> {
        if self.index < self.slice.len() {
            let result = self.slice[self.index];
            self.index += 1;
            Some(result)
        } else {
            None
        }
    }
    #[inline]
    pub fn next_u64(&mut self) -> Option<u64> {
        if self.index + 8 < self.slice.len() {
            let result = u64::from_le_bytes(
                [self.slice[self.index], self.slice[self.index + 1], self.slice[self.index + 2], self.slice[self.index + 3],
                    self.slice[self.index + 4], self.slice[self.index + 5], self.slice[self.index + 6], self.slice[self.index + 7]]);
            self.index += 8;
            Some(result)
        } else if self.index + 8 < self.slice.len() {
            let mut v: [u8; 8] = [0; 8];
            let mut i = 0;
            while i < self.slice.len() {
                v[i] = self.slice[self.index + i];
            }
            self.index += i + 1;
            Some(u64::from_le_bytes(v))
        } else {
            None
        }
    }
    #[inline]
    pub fn peek(&self) -> Option<u8> {
        if self.index < self.slice.len() {
            Some(self.slice[self.index])
        } else {
            None
        }
    }
    #[inline]
    pub fn slice_from(&self, start: usize) -> &'a [u8] {
        &self.slice[start..self.index]
    }
    #[inline]
    pub fn is_at_end(&self) -> bool {
        self.index >= self.slice.len()
    }

    #[inline]
    pub fn match_pattern(&mut self, pattern: &[u8]) -> bool {
        let end = self.index + pattern.len();
        if end <= self.slice.len() && self.slice[self.index..end] == *pattern {
            self.index += pattern.len();
            true
        } else {
            false
        }
    }

    pub fn data(&self) -> &'a [u8] {
        self.slice
    }
}


pub struct Lexer<'a> {
    reader: SliceRead<'a>,
}


const MASK_OPEN_CURLY: u64 = 0x0101010101010101 * b'{' as u64;
const MASK_OPEN_CURLY_SINGLE: u8 = 0x01 * b'{' as u8;
const MASK_CLOSE_CURLY: u64 = 0x0101010101010101 * b'}' as u64;
const MASK_CLOSE_CURLY_SINGLE: u8 = 0x01 * b'}' as u8;
const MASK_OPEN_SQUARE: u64 = 0x0101010101010101 * b'[' as u64;
const MASK_OPEN_SQUARE_SINGLE: u8 = 0x01 * b'[' as u8;
const MASK_CLOSE_SQUARE: u64 = 0x0101010101010101 * b']' as u64;
const MASK_CLOSE_SQUARE_SINGLE: u8 = 0x01 * b']' as u8;

impl<'a> Lexer<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Lexer {
            reader: SliceRead::new(input),
        }
    }

    pub fn consume_string_until_end_of_array(&mut self) -> Option<&'a str> {
        let mut square_close_count = 1;
        let start = self.reader.index - 1;
        while !self.reader.is_at_end() {
            let current_index = self.reader.index;
            if let Some(bytes) = self.reader.next_u64() {
                let comparison = MASK_CLOSE_SQUARE ^ bytes;
                let high_bit_mask1 = (((comparison >> 1) | 0x8080808080808080) - comparison) & 0x8080808080808080;
                if high_bit_mask1 == 0 {
                    let comparison = MASK_OPEN_SQUARE ^ bytes;
                    let high_bit_mask1 = (((comparison >> 1) | 0x8080808080808080) - comparison) & 0x8080808080808080;
                    if high_bit_mask1 == 0 {
                        continue;
                    } else {
                        self.reader.index = current_index + (high_bit_mask1.trailing_zeros() >> 3) as usize;
                    }
                } else {
                    self.reader.index = current_index + (high_bit_mask1.trailing_zeros() >> 3) as usize;
                }
            }
            match self.reader.next()? {
                b'[' => square_close_count += 1,
                b']' => {
                    if square_close_count == 1 {
                        return Some(string_from_bytes(&self.reader.slice[start..self.reader.index - 1])?);
                    } else {
                        square_close_count -= 1;
                    }
                }
                _ => {}
            }
        }
        None
    }

    pub fn reader_index(&self) -> usize {
        self.reader.index
    }
    pub fn reader(&mut self) -> &SliceRead<'a> {
        &self.reader
    }

    pub fn set_reader_index(&mut self, index: usize) {
        self.reader.index = index;
    }

    pub fn consume_string_until_end_of_object(&mut self) -> Option<&'a str> {
        let mut square_close_count = 1;
        let start = self.reader.index - 1;
        while !self.reader.is_at_end() {
            let current_index = self.reader.index;
            if let Some(bytes) = self.reader.next_u64() {
                let comparison = MASK_CLOSE_CURLY ^ bytes;
                let high_bit_mask1 = (((comparison >> 1) | 0x8080808080808080) - comparison) & 0x8080808080808080;
                if high_bit_mask1 == 0 {
                    let comparison = MASK_OPEN_CURLY ^ bytes;
                    let high_bit_mask1 = (((comparison >> 1) | 0x8080808080808080) - comparison) & 0x8080808080808080;
                    if high_bit_mask1 == 0 {
                        continue;
                    } else {
                        self.reader.index = current_index + (high_bit_mask1.trailing_zeros() >> 3) as usize;
                    }
                } else {
                    self.reader.index = current_index + (high_bit_mask1.trailing_zeros() >> 3) as usize;
                }
            }

            match self.reader.next()? {
                b'{' => square_close_count += 1,
                b'}' => {
                    if square_close_count == 1 {
                        let value = string_from_bytes(&self.reader.slice[start..self.reader.index])?;
                        return Some(value);
                    } else {
                        square_close_count -= 1;
                    }
                }
                _ => {}
            }
        }
        None
    }
    #[inline]
    pub fn next_token(&mut self) -> Option<Token<'a>> {
        loop {
            match self.reader.next()? {
                b'{' => return Some(Token::CurlyOpen),
                b'}' => return Some(Token::CurlyClose),
                b'[' => return Some(Token::SquareOpen),
                b']' => return Some(Token::SquareClose),
                b',' => return Some(Token::Comma),
                b':' => return Some(Token::Colon),
                b'-' | b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9' => {
                    let start = self.reader.index - 1;
                    while let Some(b) = self.reader.next() {
                        if !((b >= 0x30 && b <= 0x39) || b == b'.') {
                            break;
                        }
                    }
                    self.reader.index -= 1;
                    let s = string_from_bytes(&self.reader.slice[start..self.reader.index])?;
                    return Some(Token::Number(s))
                }
                b'"' => {
                    let start = self.reader.index;
                    while let Some(b) = self.reader.next() {
                        if b == b'"' && self.reader.slice[self.reader.index - 2] != b'\\' {
                            break; // End of string unless escaped
                        }
                    }
                    let s = string_from_bytes(&self.reader.slice[start..self.reader.index - 1])?;
                    return Some(Token::String(s))
                }
                b't' if self.reader.match_pattern(b"rue") => return Some(Token::Boolean(true)),
                b'f' if self.reader.match_pattern(b"alse") => return Some(Token::Boolean(false)),
                b'n' if self.reader.match_pattern(b"ull") => return Some(Token::Null),
                // Handle numbers, errors, etc.
                _ => {},
            }
        }
        None
    }
}


