use oxc_allocator::{Allocator, Vec as ArenaVec};
use oxc_ast::{
  AstBuilder, Comment,
  ast::{Directive, Program, Statement},
};
use oxc_diagnostics::OxcDiagnostic;
use oxc_parser::ParseOptions;
use oxc_span::SourceType;
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
  empty_str: String,
  ast: AstBuilder<'a>,

  module_record: ModuleRecord<'a>,
  source_type: SourceType,
  comments: ArenaVec<'a, Comment>,
  errors: Vec<OxcDiagnostic>,

  setup: ArenaVec<'a, Statement<'a>>,
  statements: JavaScriptBody<'a>,
  sfc_struct_jsx_statement: Option<Statement<'a>>,
}

impl<'a> ParserImpl<'a> {
  /// Create a [`ParserImpl`]
  pub fn new(allocator: &'a Allocator, source_text: &'a str, options: ParseOptions) -> Self {
    let ast = AstBuilder::new(allocator);
    Self {
      allocator,
      source_text,
      ast,
      source_type: SourceType::jsx(),
      comments: ast.vec(),

      module_record: ModuleRecord::new(allocator),
      errors: vec![],
      empty_str: " ".repeat(source_text.len()),
      options,

      setup: ast.vec(),
      statements: JavaScriptBody::new(ast.vec(), ast.vec()),
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
  pub fn pad_source(&self, source: &str, start: usize) -> String {
    format!("{}{source}", &self.empty_str[..start])
  }
}

pub struct JavaScriptBody<'a> {
  directives: ArenaVec<'a, Directive<'a>>,
  statements: ArenaVec<'a, Statement<'a>>,
}

impl<'a> JavaScriptBody<'a> {
  const fn new(
    directives: ArenaVec<'a, Directive<'a>>,
    statements: ArenaVec<'a, Statement<'a>>,
  ) -> Self {
    Self { directives, statements }
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
