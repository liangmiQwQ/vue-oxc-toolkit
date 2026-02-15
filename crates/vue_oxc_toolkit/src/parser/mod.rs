use oxc_allocator::{Allocator, Vec as ArenaVec};
use oxc_ast::{
  AstBuilder, Comment,
  ast::{Directive, Program, Statement},
};
use oxc_diagnostics::OxcDiagnostic;
use oxc_parser::ParseOptions;
use oxc_span::{SourceType, Span};
use oxc_syntax::module_record::ModuleRecord;

mod elements;
mod error;
mod modules;
mod parse;
mod script;

pub struct ParserImpl<'a> {
  allocator: &'a Allocator,
  source_text: &'a str,
  options: ParseOptions,

  comments: ArenaVec<'a, Comment>,
  source_type: SourceType,
  module_record: ModuleRecord<'a>,
  errors: Vec<OxcDiagnostic>,

  empty_str: String,
  ast: AstBuilder<'a>,
  script_set: bool,
  setup_set: bool,
  script_tags: Vec<Span>,

  directives: ArenaVec<'a, Directive<'a>>,
  statements: ArenaVec<'a, Statement<'a>>,
  setup: ArenaVec<'a, Statement<'a>>,
  sfc_struct_jsx_statement: Option<Statement<'a>>,
}

impl<'a> ParserImpl<'a> {
  /// Create a [`ParserImpl`]
  pub fn new(allocator: &'a Allocator, source_text: &'a str, options: ParseOptions) -> Self {
    let ast = AstBuilder::new(allocator);
    Self {
      allocator,
      source_text,
      options,

      comments: ast.vec(),
      source_type: SourceType::jsx(),
      module_record: ModuleRecord::new(allocator),
      errors: vec![],

      empty_str: " ".repeat(source_text.len()),
      ast,
      script_set: false,
      setup_set: false,
      script_tags: vec![],

      directives: ast.vec(),
      statements: ast.vec(),
      setup: ast.vec(),
      sfc_struct_jsx_statement: None,
    }
  }
}

pub struct ParserImplReturn<'a> {
  pub program: Program<'a>,
  pub module_record: ModuleRecord<'a>,

  pub fatal: bool,
  pub errors: Vec<OxcDiagnostic>,
}

// Some public utils
impl<'a> ParserImpl<'a> {
  pub fn oxc_parse(
    &mut self,
    source: &str,
    start: usize,
  ) -> Option<(ArenaVec<'a, Directive<'a>>, ArenaVec<'a, Statement<'a>>, ModuleRecord<'a>)> {
    let source_text = self.ast.atom(&self.pad_source(source, start));
    let mut ret = oxc_parser::Parser::new(self.allocator, source_text.as_str(), self.source_type)
      .with_options(self.options)
      .parse();

    self.errors.append(&mut ret.errors);
    if ret.panicked {
      None
    } else {
      self.comments.append(&mut ret.program.comments);
      Some((ret.program.directives, ret.program.body, ret.module_record))
    }
  }

  /// A workaround
  /// Use placeholder to make the location AST returned correct
  fn pad_source(&self, source: &str, start: usize) -> String {
    format!("{}{source}", &self.empty_str[..start])
  }
}

#[macro_export]
macro_rules! is_void_tag {
  ($name:ident) => {
    matches!(
      $name,
      "area"
        | "base"
        | "br"
        | "col"
        | "embed"
        | "hr"
        | "img"
        | "input"
        | "link"
        | "meta"
        | "param"
        | "source"
        | "track"
        | "wbr"
    )
  };
}

/// For inner parser implement use. Use Result<T, ()> for fn which may make parser panic
type ResParse<T> = Result<T, ()>;

trait ResParseExt<T> {
  fn panic() -> ResParse<T> {
    Err(())
  }

  // do not use `ok` as name, because it is a method of Result
  fn success(t: T) -> ResParse<T> {
    Ok(t)
  }
}

impl<T> ResParseExt<T> for ResParse<T> {}
