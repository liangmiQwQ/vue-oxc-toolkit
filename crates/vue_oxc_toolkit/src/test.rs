#[macro_export]
macro_rules! test_ast {
  ($file_path:expr) => {
    test_ast!($file_path, false);
  };
  ($file_path:expr, $should_panic:expr) => {{
    use insta::assert_snapshot;
    use oxc_allocator::Allocator;
    use oxc_parser::ParseOptions;

    use crate::parser::ParserImpl;
    use crate::test::read_file;

    let allocator = Allocator::default();
    let source_text = read_file($file_path);

    let ret = ParserImpl::new(&allocator, &source_text, ParseOptions::default()).parse();

    let result = format!("Program: {:#?} \n Errors: {:#?}", ret.program, ret.errors);
    assert_eq!(ret.fatal, $should_panic);
    assert_snapshot!(result);
  }};
}

pub fn read_file(file_path: &str) -> String {
  std::fs::read_to_string(format!("fixtures/{}", file_path)).expect("Failed to read test file")
}
