use std::path::Path;

use oxc_allocator::Allocator;
use oxc_ast::AstKind;
use oxc_ast_visit::Visit;
use oxc_codegen::Codegen;
use oxc_parser::ParseOptions;

use crate::{ParseConfig, parser::ParserImpl};

/// Tests for codegen
/// For downstream use
#[test]
fn validate_all_codegen_syntax() {
  let result = validate_codegen_fixtures(CodegenValidationMode::Syntax);

  if !result.syntax_errors.is_empty() {
    println!("Invalid codegen syntax in:");
    for (file_path, errors) in &result.syntax_errors {
      let snap_name = super::snapshot_name(file_path);
      println!("  {file_path}  (src/snapshots/codegen/{snap_name}.snap)");
      for error in errors {
        println!("{error}");
      }
    }
  }

  let invalid_files =
    result.syntax_errors.iter().map(|(file_path, _)| file_path).collect::<Vec<_>>();
  assert!(result.syntax_errors.is_empty(), "Invalid codegen syntax in: {invalid_files:?}");
}

#[test]
#[ignore = "Exploratory check for mapping through codegen reparse."]
fn validate_codegen_reparse_ast_structure() {
  let result = validate_codegen_fixtures(CodegenValidationMode::Structure);

  assert!(
    result.ast_diffs.is_empty(),
    "Codegen reparse AST mismatch:\n{}",
    result.ast_diffs.join("\n")
  );
}

#[derive(Clone, Copy)]
enum CodegenValidationMode {
  Syntax,
  Structure,
}

#[derive(Default)]
struct CodegenValidationResult {
  syntax_errors: Vec<(String, Vec<String>)>,
  ast_diffs: Vec<String>,
}

fn validate_codegen_fixtures(mode: CodegenValidationMode) -> CodegenValidationResult {
  let mut result = CodegenValidationResult::default();
  visit_codegen_fixtures(Path::new("fixtures"), mode, &mut result);
  result
}

fn visit_codegen_fixtures(
  path: &Path,
  mode: CodegenValidationMode,
  result: &mut CodegenValidationResult,
) {
  for entry in std::fs::read_dir(path).unwrap() {
    let entry = entry.unwrap();
    let path = entry.path();

    if path.is_dir() {
      visit_codegen_fixtures(&path, mode, result);
    } else if path.extension().and_then(|s| s.to_str()) == Some("vue") {
      validate_codegen_fixture(&path, mode, result);
    }
  }
}

fn validate_codegen_fixture(
  path: &Path,
  mode: CodegenValidationMode,
  result: &mut CodegenValidationResult,
) {
  let file_path = path.strip_prefix("fixtures").unwrap().to_str().unwrap().trim_start_matches('/');
  let source_text = std::fs::read_to_string(path).unwrap();
  let allocator = Allocator::default();
  let ret = ParserImpl::new(
    &allocator,
    &source_text,
    ParseOptions::default(),
    ParseConfig { codegen: true },
  )
  .parse();
  if ret.fatal {
    return;
  }

  let codegen = Codegen::new().build(&ret.program).code;
  if matches!(mode, CodegenValidationMode::Syntax) {
    assert_codegen_snapshot(file_path, &codegen);
  }

  let new_allocator = Allocator::default();
  let reparsed = oxc_parser::Parser::new(&new_allocator, &codegen, ret.program.source_type)
    .with_options(ParseOptions::default())
    .parse();
  if !reparsed.errors.is_empty() {
    result
      .syntax_errors
      .push((file_path.to_string(), reparsed.errors.iter().map(ToString::to_string).collect()));
    return;
  }

  if matches!(mode, CodegenValidationMode::Structure) {
    let original_structure = ast_structure(&ret.program);
    let reparsed_structure = ast_structure(&reparsed.program);
    if original_structure != reparsed_structure {
      result.ast_diffs.push(format_ast_structure_diff(
        file_path,
        &original_structure,
        &reparsed_structure,
      ));
    }
  }
}

fn assert_codegen_snapshot(file_path: &str, codegen: &str) {
  let snap_name = super::snapshot_name(file_path);
  let mut settings = insta::Settings::clone_current();
  settings.set_snapshot_path("snapshots/codegen");
  settings.set_prepend_module_to_snapshot(false);
  settings.bind(|| {
    insta::assert_snapshot!(snap_name, codegen);
  });
}

struct AstStructureCollector {
  nodes: Vec<String>,
}

impl AstStructureCollector {
  fn new() -> Self {
    Self { nodes: Vec::new() }
  }
}

impl<'a> Visit<'a> for AstStructureCollector {
  fn enter_node(&mut self, kind: AstKind<'a>) {
    let kind = format!("{kind:?}");
    let kind = match memchr::memchr(b'(', kind.as_bytes()) {
      Some(index) => kind[..index].to_owned(),
      None => kind,
    };
    self.nodes.push(kind);
  }
}

fn ast_structure(program: &oxc_ast::ast::Program) -> Vec<String> {
  let mut collector = AstStructureCollector::new();
  collector.visit_program(program);
  collector.nodes
}

fn format_ast_structure_diff(file_path: &str, original: &[String], reparsed: &[String]) -> String {
  let first_diff =
    original.iter().zip(reparsed.iter()).position(|(original, reparsed)| original != reparsed);

  let Some(index) = first_diff else {
    return format!(
      "{file_path}: common prefix matched, len {} vs {}",
      original.len(),
      reparsed.len()
    );
  };

  format!(
    "{file_path}: first diff at node {index}: original={}, reparsed={} (len {} vs {})",
    original[index],
    reparsed[index],
    original.len(),
    reparsed.len(),
  )
}
