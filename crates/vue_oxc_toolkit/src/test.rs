#[macro_export]
macro_rules! test_ast {
  ($file_path:expr) => {
    test_ast!($file_path, false, false);
  };
  ($file_path:expr, $should_errors:expr, $should_panic:expr) => {{
    use insta::assert_snapshot;
    use oxc_allocator::Allocator;
    use oxc_codegen::Codegen;
    use oxc_parser::ParseOptions;

    use crate::parser::ParserImpl;
    use crate::test::read_file;

    let allocator = Allocator::default();
    let source_text = read_file($file_path);

    let ret = ParserImpl::new(&allocator, &source_text, ParseOptions::default()).parse();
    let js = Codegen::new().build(&ret.program);

    let result = format!(
      "Program: {:#?}\nErrors: {:#?}\nJS: {}",
      ret.program, ret.errors, js.code
    );
    assert_eq!(ret.fatal, $should_panic);
    assert_snapshot!(result);
  }};
}

pub fn read_file(file_path: &str) -> String {
  std::fs::read_to_string(format!("fixtures/{}", file_path)).expect("Failed to read test file")
}
