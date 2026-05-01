use crate::{
  VueJsxCodegen,
  codegen::Mapping,
  parser::{ParseConfig, ParserImpl},
  test::{read_file, snapshot_name},
};
use oxc_allocator::Allocator;
use oxc_ast::{AstKind, ast::Program};
use oxc_codegen::Codegen;
use oxc_parser::ParseOptions;

pub fn format_program_codegen(program: &Program) -> String {
  Codegen::new().build(program).code
}

use oxc_ast_visit::Visit;
use oxc_span::{ContentEq, GetSpan, SPAN, Span};

pub fn run_codegen_test(file_path: &str) {
  let source_text = read_file(file_path);
  let ret = VueJsxCodegen::new(&source_text).build();
  assert!(!ret.panicked, "Codegen unexpectedly panicked for {file_path}");

  let snap_name = snapshot_name(file_path);
  let mut settings = insta::Settings::clone_current();
  settings.set_snapshot_path("snapshots/codegen");
  settings.set_prepend_module_to_snapshot(false);
  settings.bind(|| {
    insta::assert_snapshot!(snap_name, ret.source_text);
  });

  let allocator = Allocator::default();
  let reparsed = oxc_parser::Parser::new(&allocator, &ret.source_text, ret.source_type)
    .with_options(ParseOptions::default())
    .parse();
  assert!(
    reparsed.errors.is_empty(),
    "Invalid codegen syntax in {file_path}: {:#?}",
    reparsed.errors,
  );

  assert_reparsed_codegen_ast(file_path, &source_text, &reparsed.program, &ret.mappings);
}

fn assert_reparsed_codegen_ast(
  file_path: &str,
  source_text: &str,
  reparsed_program: &oxc_ast::ast::Program<'_>,
  mappings: &[Mapping],
) {
  let allocator = Allocator::default();
  let ret = ParserImpl::new(
    &allocator,
    source_text,
    ParseOptions::default(),
    ParseConfig { codegen: true },
  )
  .parse();

  assert!(!ret.fatal, "Codegen parser unexpectedly panicked for {file_path}");
  program_codegen_eq(&ret.program, reparsed_program, mappings, file_path);
}

fn program_codegen_eq(origin: &Program, reparsed: &Program, mappings: &[Mapping], file_path: &str) {
  assert!(origin.hashbang.content_eq(&reparsed.hashbang), "Hashbang differs for {file_path}");
  assert!(origin.directives.content_eq(&reparsed.directives), "Directives differs for {file_path}");
  assert!(origin.body.content_eq(&reparsed.body), "Body differs for {file_path}");

  let origin_spans = collect_spans(origin);
  let reparsed_spans = collect_spans(reparsed);
  origin_spans.into_iter().zip(reparsed_spans).for_each(|(origin, reparsed)| {
    assert_eq!(origin.0, reparsed.0, "[MAPPING] Node kind differs for {file_path}");

    if origin.1 == SPAN {
      return;
    }

    if !mappings.iter().any(|mapping| mapping.original_span == origin.1) {
      return;
    }

    assert!(
      mappings.iter().any(|mapping| {
        mapping.original_span == origin.1 && spans_overlap(mapping.codegen_span, reparsed.1)
      }),
      "[MAPPING] Missing span for {file_path}: {origin:?} -> {reparsed:?}",
    );
  });
}

fn spans_overlap(a: Span, b: Span) -> bool {
  a.start < b.end && b.start < a.end
}

#[test]
fn clean_script_statements_are_raw_copied() {
  let source_text = read_file("scripts/codegen_fidelity.vue");
  let ret = VueJsxCodegen::new(&source_text).build();

  assert!(ret.source_text.contains("import { foo as foo, bar as baz } from './dep'"));
  assert!(ret.source_text.contains("const decimal = 1.0"));
  assert!(ret.source_text.contains("const escaped = '\\x41'"));
  assert!(ret.source_text.contains("const attrs = <div label={'\\x42'} raw=\"\\x43\" />"));
}

#[test]
fn clean_script_mappings_cover_raw_statement_segments() {
  let source_text = read_file("scripts/codegen_fidelity.vue");
  let ret = VueJsxCodegen::new(&source_text).build();
  let statement = "const decimal = 1.0";
  let original_span = find_span(&source_text, statement);
  let codegen_span = find_span(&ret.source_text, statement);

  assert!(ret.mappings.iter().any(|mapping| {
    mapping.original_span == original_span && mapping.codegen_span == codegen_span
  }));
}

#[test]
fn synthetic_wrappers_do_not_map_to_the_whole_sfc() {
  let source_text = read_file("scripts/mapping.vue");
  let ret = VueJsxCodegen::new(&source_text).build();
  let whole_sfc = Span::new(0, source_text.len() as u32);

  assert!(!ret.mappings.iter().any(|mapping| mapping.original_span == whole_sfc));
}

#[test]
fn dirty_template_boundary_owns_one_mapping() {
  let source_text = "<template><div>{{ msg }}</div></template><script setup>const msg = 1</script>";
  let ret = VueJsxCodegen::new(source_text).build();
  let original_span = find_span(source_text, "<template><div>{{ msg }}</div></template>");
  let mappings = ret
    .mappings
    .iter()
    .filter(|mapping| mapping.original_span == original_span)
    .collect::<Vec<_>>();

  assert_eq!(mappings.len(), 1);
  assert_eq!(
    mappings[0].codegen_span.source_text(&ret.source_text),
    "<template><div>{msg}</div></template>",
  );
}

#[test]
fn generated_diagnostics_remap_to_dirty_vue_boundary() {
  let source_text = "<template><div>{{ msg }}</div></template><script setup>const msg = 1</script>";
  let ret = VueJsxCodegen::new(source_text).build();
  let diagnostic_start = find_span(&ret.source_text, "{msg}").start + 1;
  let mapping = ret
    .mappings
    .iter()
    .filter(|mapping| {
      mapping.codegen_span.start <= diagnostic_start && diagnostic_start < mapping.codegen_span.end
    })
    .min_by_key(|mapping| mapping.codegen_span.size())
    .expect("diagnostic should resolve through a mapping");

  assert_eq!(
    mapping.original_span,
    find_span(source_text, "<template><div>{{ msg }}</div></template>"),
  );
}

fn find_span(source_text: &str, needle: &str) -> Span {
  let start = source_text.find(needle).expect("test fixture should contain needle") as u32;
  Span::sized(start, needle.len() as u32)
}

fn collect_spans(program: &Program) -> Vec<(String, Span)> {
  let mut collector = SpanCollector { spans: Vec::new() };
  collector.visit_program(program);
  collector.spans
}

struct SpanCollector {
  spans: Vec<(String, Span)>,
}

impl<'a> Visit<'a> for SpanCollector {
  fn enter_node(&mut self, kind: oxc_ast::AstKind<'a>) {
    // ExpressionStatement is excluded because the parser's Program mapping
    // (codegen_span = 0..total) coincides with the reparsed ExpressionStatement
    // span when the statement is the sole top-level node, causing SpanMapper to
    // wrongly assign it the program's original_span instead of SPAN.
    if matches!(kind, AstKind::ExpressionStatement(_)) {
      return;
    }

    let kind_str = format!("{kind:?}");
    let kind_name = match memchr::memchr(b'(', kind_str.as_bytes()) {
      Some(index) => kind_str[..index].to_owned(),
      None => kind_str,
    };
    self.spans.push((kind_name, kind.span()));
  }
}
