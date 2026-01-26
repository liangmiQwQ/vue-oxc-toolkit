use crate::parser::ParserImpl;
pub use crate::parser::ParserImplReturn;
use oxc_allocator::Allocator;
use oxc_parser::ParseOptions;

#[macro_export]
macro_rules! test_ast {
  ($file_path:expr) => {
    test_ast!($file_path, false, false);
  };
  ($file_path:expr, $should_errors:expr, $should_panic:expr) => {{
    $crate::test::run_test($file_path, "ast", |ret| {
      use oxc_codegen::Codegen;
      let js = Codegen::new().build(&ret.program);
      assert_eq!(ret.fatal, $should_panic);
      format!("Program: {:#?}\nErrors: {:#?}\nJS: {}", ret.program, ret.errors, js.code)
    });
  }};
}

#[macro_export]
macro_rules! test_module_record {
  ($file_path:expr) => {{
    $crate::test::run_test($file_path, "module_record", |ret| {
      format!("Module Record: {:#?}", ret.module_record)
    });
  }};
}

pub fn run_test<F>(file_path: &str, folder: &str, f: F)
where
  F: for<'a> FnOnce(&ParserImplReturn<'a>) -> String,
{
  let allocator = Allocator::default();
  let source_text = read_file(file_path);

  let ret = ParserImpl::new(&allocator, &source_text, ParseOptions::default()).parse();

  let result = f(&ret);

  let snapshot_name = file_path.replace(['/', '.'], "_");
  let mut settings = insta::Settings::clone_current();
  settings.set_snapshot_path(format!("parser/snapshots/{folder}"));
  settings.set_prepend_module_to_snapshot(false);
  settings.bind(|| {
    insta::assert_snapshot!(snapshot_name, result);
  });
}

pub fn read_file(file_path: &str) -> String {
  std::fs::read_to_string(format!("fixtures/{file_path}")).expect("Failed to read test file")
}
