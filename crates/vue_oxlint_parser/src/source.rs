//! Byte cursor over the SFC source.
//!
//! The lexer drives a `Source` forward through `&[u8]`. All offsets are byte
//! indices into the original source; the lexer never decodes UTF-8 because
//! every syntactic delimiter Vue cares about is single-byte ASCII.

pub struct Source<'a> {
  bytes: &'a [u8],
  pos: u32,
}

impl<'a> Source<'a> {
  #[must_use]
  pub const fn new(text: &'a str) -> Self {
    Self { bytes: text.as_bytes(), pos: 0 }
  }

  #[inline]
  #[must_use]
  pub const fn pos(&self) -> u32 {
    self.pos
  }

  #[inline]
  #[must_use]
  pub const fn len(&self) -> u32 {
    self.bytes.len() as u32
  }

  #[inline]
  #[must_use]
  pub const fn is_empty(&self) -> bool {
    self.bytes.is_empty()
  }

  #[inline]
  #[must_use]
  pub const fn is_eof(&self) -> bool {
    self.pos >= self.bytes.len() as u32
  }

  #[inline]
  pub const fn seek(&mut self, pos: u32) {
    self.pos = pos;
  }

  #[inline]
  pub const fn advance(&mut self, n: u32) {
    self.pos += n;
  }

  #[inline]
  #[must_use]
  pub fn peek(&self) -> Option<u8> {
    self.bytes.get(self.pos as usize).copied()
  }

  #[inline]
  #[must_use]
  pub fn peek_at(&self, offset: u32) -> Option<u8> {
    self.bytes.get((self.pos + offset) as usize).copied()
  }

  #[inline]
  #[must_use]
  pub fn rest(&self) -> &'a [u8] {
    &self.bytes[self.pos as usize..]
  }

  #[inline]
  #[must_use]
  pub const fn bytes(&self) -> &'a [u8] {
    self.bytes
  }

  #[inline]
  #[must_use]
  pub fn slice(&self, lo: u32, hi: u32) -> &'a [u8] {
    &self.bytes[lo as usize..hi as usize]
  }

  #[inline]
  #[must_use]
  pub fn starts_with(&self, needle: &[u8]) -> bool {
    self.rest().starts_with(needle)
  }

  /// Case-insensitive prefix check. `needle` must already be lowercase.
  #[must_use]
  pub fn starts_with_ascii_ci(&self, needle: &[u8]) -> bool {
    let r = self.rest();
    if r.len() < needle.len() {
      return false;
    }
    r[..needle.len()].eq_ignore_ascii_case(needle)
  }
}
