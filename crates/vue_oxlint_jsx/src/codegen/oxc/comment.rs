//! Comment-printing stubs.
//!
//! The vendored codegen disables all comment output, so these helpers exist
//! purely as no-op shims to keep call sites in `gen.rs` simple.

use oxc_ast::Comment;

use super::Codegen;

#[allow(clippy::unused_self, clippy::needless_pass_by_ref_mut)]
impl Codegen<'_> {
  #[inline]
  pub(crate) const fn has_comment(&self, _start: u32) -> bool {
    false
  }

  #[inline]
  pub(crate) fn print_leading_comments(&mut self, _start: u32) {}

  #[inline]
  pub(crate) const fn get_comments(&mut self, _start: u32) -> Option<Vec<Comment>> {
    None
  }

  #[inline]
  pub(crate) fn print_comments_at(&mut self, _start: u32) {}

  #[inline]
  pub(crate) const fn has_legal_orphans_before(&self, _end: u32) -> bool {
    false
  }

  #[inline]
  pub(crate) fn print_legal_orphans_before(&mut self, _end: u32) {}

  #[inline]
  pub(crate) const fn print_comments_in_range(&mut self, _start: u32, _end: u32) -> bool {
    false
  }

  #[inline]
  pub(crate) const fn print_expr_comments(&mut self, _start: u32) -> bool {
    false
  }

  #[inline]
  pub(crate) fn print_comments(&mut self, _comments: &[Comment]) {}
}
