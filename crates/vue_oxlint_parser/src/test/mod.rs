macro_rules! test_sfc {
  ($test_name:ident, $file_path:expr) => {
    #[test]
    fn $test_name() {
      let allocator = oxc_allocator::Allocator::default();
      let source_text = $crate::test::read_fixture($file_path);
      let ret = $crate::parse_sfc(&allocator, &source_text);

      assert!(!ret.panicked, "fixture {} panicked: {:?}", $file_path, ret.errors);
      assert!(ret.errors.is_empty(), "fixture {} returned errors: {:?}", $file_path, ret.errors);
    }
  };
}

pub(crate) use test_sfc;

pub(crate) fn read_fixture(file_path: &str) -> String {
  std::fs::read_to_string(format!("fixtures/{file_path}"))
    .unwrap_or_else(|err| panic!("failed to read fixture {file_path}: {err}"))
}
