#[cfg(test)]
mod tests {
  use crate::parser::ParserImpl;
  use oxc_allocator::Allocator;
  use oxc_parser::ParseOptions;

  #[test]
  fn test_small_offset_interpolation() {
    let allocator = Allocator::default();
    let source = "{{a}}";
    let parser = ParserImpl::new(&allocator, source, ParseOptions::default());
    let ret = parser.parse();
    assert!(!ret.fatal);
  }

  #[test]
  fn test_small_offset_script() {
    let allocator = Allocator::default();
    let source = "<script>console.log(1)</script>";
    let parser = ParserImpl::new(&allocator, source, ParseOptions::default());
    let ret = parser.parse();
    assert!(!ret.fatal);
  }

  #[test]
  fn test_small_offset_shorthand_property() {
    let allocator = Allocator::default();
    let source = "<div :[foo]=\"bar\"></div>";
    let parser = ParserImpl::new(&allocator, source, ParseOptions::default());
    let ret = parser.parse();
    assert!(!ret.fatal);
  }

  #[test]
  fn test_no_closing_tag_underflow() {
    let allocator = Allocator::default();
    let source = "<div>";
    let parser = ParserImpl::new(&allocator, source, ParseOptions::default());
    let ret = parser.parse();
    assert!(!ret.fatal);
  }
}
