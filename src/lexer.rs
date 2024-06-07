use crate::string_from_bytes;

#[derive(Debug)]
pub enum Token<'json> {
    CurlyOpen,
    CurlyClose,
    SquareOpen,
    SquareClose,
    Colon,
    Comma,
    String(&'json str),
    Number(&'json str),
    Boolean(&'json str),
    Null,
}


pub struct SliceRead<'json> {
    slice: &'json [u8],
    index: usize,
}

impl<'json> SliceRead<'json> {
    pub fn new(slice: &'json [u8]) -> Self {
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
    pub fn next_u64(&mut self) -> (u64, usize) {
        if self.index + 8 < self.slice.len() {
            let result = u64::from_le_bytes(
                [self.slice[self.index], self.slice[self.index + 1], self.slice[self.index + 2], self.slice[self.index + 3],
                    self.slice[self.index + 4], self.slice[self.index + 5], self.slice[self.index + 6], self.slice[self.index + 7]]);
            self.index += 8;
            (result, 8)
        } else {
            let mut v: [u8; 8] = [0; 8];
            let mut i = 0;
            while self.index + i < self.slice.len() {
                v[i] = self.slice[self.index + i];
                i += 1;
            }
            self.index += i;
            (u64::from_le_bytes(v), i)
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
    pub fn slice_from(&self, start: usize) -> &'json [u8] {
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

    pub fn data(&self) -> &'json [u8] {
        self.slice
    }
}


pub struct Lexer<'json> {
    reader: SliceRead<'json>,
}


const MASK_OPEN_CURLY: u64 = 0x0101010101010101 * b'{' as u64;
const MASK_CLOSE_CURLY: u64 = 0x0101010101010101 * b'}' as u64;
const MASK_OPEN_SQUARE: u64 = 0x0101010101010101 * b'[' as u64;
const MASK_CLOSE_SQUARE: u64 = 0x0101010101010101 * b']' as u64;
const MASK_QUOTE: u64 = 0x0101010101010101 * b'"' as u64;

impl<'json> Lexer<'json> {
    pub fn new(input: &'json [u8]) -> Self {
        Lexer {
            reader: SliceRead::new(input),
        }
    }

    #[inline]
    pub fn consume_string_until_end_of_array(&mut self, array_start_index: usize) -> Option<&'json str> {
        let mut square_close_count = 1;
        while !self.reader.is_at_end() {
            let current_index = self.reader.index;
            let (bytes, _) = self.reader.next_u64();
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
            match self.reader.next()? {
                b'[' => square_close_count += 1,
                b']' => {
                    if square_close_count == 1 {
                        return Some(string_from_bytes(&self.reader.slice[array_start_index..self.reader.index])?);
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
    pub fn reader(&mut self) -> &SliceRead<'json> {
        &self.reader
    }

    pub fn set_reader_index(&mut self, index: usize) {
        self.reader.index = index;
    }

    #[inline]
    pub fn consume_string_until_end_of_object(&mut self, should_return: bool) -> Option<&'json str> {
        let mut square_close_count = 1;
        let start = self.reader.index - 1;
        while !self.reader.is_at_end() {
            let current_index = self.reader.index;
            let (bytes, _) = self.reader.next_u64();
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

            match self.reader.next()? {
                b'{' => square_close_count += 1,
                b'}' => {
                    if square_close_count == 1 {
                        if should_return {
                            let value = string_from_bytes(&self.reader.slice[start..self.reader.index])?;
                            return Some(value);
                        } else {
                            break;
                        }
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
    pub fn next_token(&mut self) -> Option<Token<'json>> {
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
                    return Some(Token::Number(s));
                }
                b'"' => {
                    let start = self.reader.index;
                    while !self.reader.is_at_end() {
                        let (bytes, read_bytes) = self.reader.next_u64();
                        let comparison = MASK_QUOTE ^ bytes;
                        let high_bit_mask1 = (((comparison >> 1) | 0x8080808080808080) - comparison) & 0x8080808080808080;
                        // println!("...{}", String::from_utf8_lossy(&self.reader.slice[self.reader.index - read_bytes..self.reader.index]));
                        if high_bit_mask1 != 0 {
                            let position = (high_bit_mask1.trailing_zeros() >> 3) as usize;
                            if self.reader.slice[self.reader.index - read_bytes + position - 1] != b'\\' {
                                self.reader.index = self.reader.index - read_bytes + position + 1;
                                break;
                            } else {
                                self.reader.index = self.reader.index - read_bytes + position + 1;
                            }
                        }
                    }
                    let s = string_from_bytes(&self.reader.slice[start..self.reader.index - 1])?;
                    return Some(Token::String(s));
                }
                b't' if self.reader.match_pattern(b"rue") => return Some(Token::Boolean(string_from_bytes(&self.reader.slice[self.reader.index-4..self.reader.index])?)),
                b'f' if self.reader.match_pattern(b"alse") => return Some(Token::Boolean(string_from_bytes(&self.reader.slice[self.reader.index-5..self.reader.index])?)),
                b'n' if self.reader.match_pattern(b"ull") => return Some(Token::Null),
                _ => {}
            }
        }
        None
    }
}


