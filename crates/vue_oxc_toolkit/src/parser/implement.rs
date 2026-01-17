use std::cell::RefCell;
use std::collections::HashSet;
use std::mem;

use oxc_allocator::{self, Allocator, Dummy, TakeIn};
use oxc_ast::ast::{
  AssignmentTarget, Expression, FormalParameterKind, JSXAttributeItem, JSXChild, JSXExpression,
  Program, PropertyKind, Statement,
};
use oxc_ast::{AstBuilder, Comment, CommentKind, NONE};
use oxc_ast_visit::VisitMut;
use oxc_diagnostics::OxcDiagnostic;
use oxc_span::{Atom, GetSpan, SPAN, SourceType, Span};
use vue_compiler_core::SourceLocation;
use vue_compiler_core::error::ErrorHandler;
use vue_compiler_core::parser::{
  AstNode, Directive, DirectiveArg, ElemProp, Element, ParseOption, Parser, SourceNode, TextNode,
  WhitespaceStrategy,
};
use vue_compiler_core::scanner::{ScanOption, Scanner};
use vue_compiler_core::util::find_prop;

use super::ParserImpl;
use super::ParserImplReturn;
use super::utils::is_simple_identifier;

struct OxcErrorHandler<'a> {
  errors: &'a RefCell<Vec<OxcDiagnostic>>,
}

impl ErrorHandler for OxcErrorHandler<'_> {
  fn on_error(&self, error: vue_compiler_core::error::CompilationError) {
    self
      .errors
      .borrow_mut()
      .push(OxcDiagnostic::error(error.to_string()));
  }
}

pub trait SourceLocatonSpan {
  fn span(&self) -> Span;
}

impl SourceLocatonSpan for SourceLocation {
  fn span(&self) -> Span {
    Span::new(self.start.offset as u32, self.end.offset as u32)
  }
}

impl<'a> ParserImpl<'a> {
  pub fn new(allocator: &'a Allocator, source_text: &'a str) -> Self {
    let ast = AstBuilder::new(allocator);
    Self {
      allocator,
      source_type: SourceType::jsx(),
      source_text,
      ast,
      comments: RefCell::from(ast.vec()),
      empty_str: ".".repeat(source_text.len()),
    }
  }

  /// # Panics
  ///
  /// Panics if the `lang` attribute is not a valid `SourceType` extension.
  pub fn parse(mut self) -> ParserImplReturn<'a> {
    let parser = Parser::new(ParseOption {
      whitespace: WhitespaceStrategy::Preserve,
      ..Default::default()
    });
    let errors = RefCell::new(vec![]);
    let scanner = Scanner::new(ScanOption::default());
    let tokens = scanner.scan(self.source_text, OxcErrorHandler { errors: &errors });
    let result = parser.parse(tokens, OxcErrorHandler { errors: &errors });
    let mut source_types: HashSet<&str> = HashSet::new();

    let ast = &self.ast;
    let mut children = ast.vec();
    for child in result.children {
      match child {
        AstNode::Element(node) => {
          if node.tag_name == "script" {
            let lang = find_prop(&node, "lang")
              .and_then(|p| match p.get_ref() {
                ElemProp::Attr(p) => p.value.as_ref().map(|value| value.content.raw),
                ElemProp::Dir(_) => None,
              })
              .unwrap_or("js");

            source_types.insert(lang);

            if source_types.len() > 1 {
              errors.borrow_mut().push(OxcDiagnostic::error(format!(
                "Multiple script tags with different languages: {source_types:?}"
              )));

              return ParserImplReturn {
                program: Program::dummy(ast.allocator),
                fatal: true,
                errors: errors.take(),
              };
            }

            self.source_type = if lang.starts_with("js") {
              SourceType::jsx()
            } else if lang.starts_with("ts") {
              SourceType::tsx()
            } else {
              errors.borrow_mut().push(OxcDiagnostic::error(format!(
                "Unsupported script language: {lang}"
              )));

              return ParserImplReturn {
                program: Program::dummy(ast.allocator),
                fatal: true,
                errors: errors.take(),
              };
            };

            let script_block = if let Some(child) = node.children.first() {
              let span = child.get_location().span();
              let source = span.source_text(self.source_text);
              oxc_parser::Parser::new(
                self.allocator,
                ast
                  .atom(&format!(
                    "/*{}*/{source}",
                    &self.empty_str[..span.start as usize - 4]
                  ))
                  .as_str(),
                SourceType::from_extension(lang).unwrap(),
              )
              .parse()
              .program
              .body
            } else {
              ast.vec()
            };
            children.push(self.parse_element(
              node,
              Some(ast.vec1(ast.jsx_child_expression_container(
                SPAN,
                JSXExpression::ArrowFunctionExpression(ast.alloc_arrow_function_expression(
                  SPAN,
                  false,
                  false,
                  NONE,
                  ast.formal_parameters(
                    SPAN,
                    FormalParameterKind::ArrowFormalParameters,
                    ast.vec(),
                    NONE,
                  ),
                  NONE,
                  ast.function_body(SPAN, ast.vec(), script_block),
                )),
              ))),
            ));
          } else if node.tag_name == "template" {
            children.push(self.parse_element(node, None));
          }
        }
        AstNode::Text(text) => children.push(self.parse_text(&text)),
        AstNode::Comment(comment) => children.push(self.parse_comment(&comment)),
        AstNode::Interpolation(interp) => children.push(self.parse_interpolation(&interp)),
      }
    }

    let program = ast.program(
      SPAN,
      self.source_type,
      self.source_text,
      self.comments.borrow_mut().take_in(ast.allocator),
      None,
      ast.vec(),
      ast.vec1(ast.statement_expression(
        SPAN,
        ast.expression_jsx_fragment(
          SPAN,
          ast.jsx_opening_fragment(SPAN),
          children,
          ast.jsx_closing_fragment(SPAN),
        ),
      )),
    );
    ParserImplReturn {
      program,
      fatal: false,
      errors: errors.take(),
    }
  }

  fn parse_children(
    &self,
    start: u32,
    end: u32,
    children: Vec<AstNode<'a>>,
  ) -> oxc_allocator::Vec<'a, JSXChild<'a>> {
    let ast = self.ast;
    if children.is_empty() {
      return ast.vec();
    }
    let mut result = self.ast.vec_with_capacity(children.len() + 2);

    if let Some(first) = children.first()
      && matches!(first, AstNode::Element(_) | AstNode::Interpolation(_))
      && start != first.get_location().start.offset as u32
    {
      let span = Span::new(start, first.get_location().start.offset as u32);
      let value = span.source_text(self.source_text);
      result.push(ast.jsx_child_text(span, value, Some(ast.atom(value))));
    }

    let last = if let Some(last) = children.last()
      && matches!(last, AstNode::Element(_) | AstNode::Interpolation(_))
      && end != last.get_location().end.offset as u32
    {
      let span = Span::new(last.get_location().end.offset as u32, end);
      let value = span.source_text(self.source_text);
      Some(ast.jsx_child_text(span, value, Some(ast.atom(value))))
    } else {
      None
    };

    result.extend(children.into_iter().map(|child| match child {
      AstNode::Element(node) => self.parse_element(node, None),
      AstNode::Text(text) => self.parse_text(&text),
      AstNode::Comment(comment) => self.parse_comment(&comment),
      AstNode::Interpolation(interp) => self.parse_interpolation(&interp),
    }));

    if let Some(last) = last {
      result.push(last);
    }

    result
  }

  fn parse_element(
    &self,
    node: Element<'a>,
    children: Option<oxc_allocator::Vec<'a, JSXChild<'a>>>,
  ) -> JSXChild<'a> {
    let ast = self.ast;

    let open_element_span = {
      let start = node.location.start.offset;
      let end = if let Some(prop) = node.properties.last() {
        self.offset(match prop {
          ElemProp::Attr(prop) => prop.location.end.offset,
          ElemProp::Dir(prop) => prop.location.end.offset,
        })
      } else {
        start + 1 + node.tag_name.len()
      } + 1;
      Span::new(start as u32, end as u32)
    };

    let location_span = node.location.span();
    let end_element_span = {
      if location_span.source_text(self.source_text).ends_with("/>") {
        node.location.span()
      } else {
        let end = node.location.end.offset;
        let start = (self.roffset(end) - node.tag_name.len() - 3) as u32;
        Span::new(start, end as u32)
      }
    };

    ast.jsx_child_element(
      location_span,
      ast.jsx_opening_element(
        open_element_span,
        ast.jsx_element_name_identifier(
          Span::new(
            open_element_span.start + 1,
            open_element_span.start + 1 + node.tag_name.len() as u32,
          ),
          ast.atom(node.tag_name),
        ),
        NONE,
        ast.vec_from_iter(
          node
            .properties
            .into_iter()
            .map(|prop| self.parse_attribute(prop)),
        ),
      ),
      if let Some(children) = children {
        children
      } else {
        self.parse_children(open_element_span.end, end_element_span.start, node.children)
      },
      if end_element_span.eq(&location_span) {
        None
      } else {
        Some(ast.jsx_closing_element(
          end_element_span,
          ast.jsx_element_name_identifier(
            Span::new(
              end_element_span.start + 2,
              end_element_span.start + 2 + node.tag_name.len() as u32,
            ),
            ast.atom(node.tag_name),
          ),
        ))
      },
    )
  }

  fn parse_attribute(&self, prop: ElemProp<'a>) -> JSXAttributeItem<'a> {
    let ast = self.ast;
    match prop {
      ElemProp::Attr(attr) => {
        let attr_end = self.roffset(attr.location.end.offset) as u32;
        let attr_span = Span::new(attr.location.start.offset as u32, attr_end);
        ast.jsx_attribute_item_attribute(
          attr_span,
          ast.jsx_attribute_name_identifier(attr.name_loc.span(), ast.atom(attr.name)),
          if let Some(value) = attr.value {
            Some(ast.jsx_attribute_value_string_literal(
              Span::new(value.location.span().start + 1, attr_end - 1),
              ast.atom(value.content.raw),
              None,
            ))
          } else {
            None
          },
        )
      }
      ElemProp::Dir(mut dir) => {
        let dir_start = dir.location.start.offset as u32;
        let dir_end = self.roffset(dir.location.end.offset) as u32;
        let head_name = dir.head_loc.span().source_text(self.source_text);
        let modifiers = mem::take(&mut dir.modifiers);
        ast.jsx_attribute_item_attribute(
          Span::new(dir_start, dir_end),
          match dir.name {
            "bind" => {
              if let Some(argument) = &dir.argument {
                if let DirectiveArg::Dynamic(_) = argument {
                  // :[foo]="bar"
                  ast.jsx_attribute_name_identifier(
                    Span::new(dir_start, dir_start + 1),
                    ast.atom(&format!("v-bind{}", Self::parse_modifiers(&modifiers))),
                  )
                } else if head_name.starts_with(':') {
                  // :foo="bar"
                  ast.jsx_attribute_name_identifier(
                    Span::new(dir_start + 1, dir.head_loc.end.offset as u32),
                    self.parse_argument(dir.argument.as_ref().unwrap(), &modifiers),
                  )
                } else {
                  // v-bind:foo="bar"
                  ast.jsx_attribute_name_namespaced_name(
                    dir.head_loc.span(),
                    ast.jsx_identifier(Span::new(dir_start, dir_start + 6), ast.atom("v-bind")),
                    ast.jsx_identifier(
                      Span::new(dir_start + 7, dir.head_loc.end.offset as u32),
                      self.parse_argument(dir.argument.as_ref().unwrap(), &modifiers),
                    ),
                  )
                }
              } else {
                // v-bind="obj"
                ast.jsx_attribute_name_identifier(
                  dir.head_loc.span(),
                  ast.atom(&format!("v-bind{}", Self::parse_modifiers(&modifiers))),
                )
              }
            }
            _ => {
              if let Some(argument) = &dir.argument {
                let namespace_end = if head_name.starts_with("v-") {
                  dir_start + 2 + dir.name.len() as u32
                } else {
                  dir_start + 1
                };
                match argument {
                  DirectiveArg::Static(arg) => ast.jsx_attribute_name_namespaced_name(
                    dir.head_loc.span(),
                    ast.jsx_identifier(
                      Span::new(dir_start, namespace_end),
                      ast.atom(&format!("v-{}", dir.name)),
                    ),
                    ast.jsx_identifier(
                      if head_name.starts_with("v-") {
                        Span::new(namespace_end + 1, namespace_end + 1 + arg.len() as u32)
                      } else {
                        Span::new(namespace_end, namespace_end + arg.len() as u32)
                      },
                      self.parse_argument(dir.argument.as_ref().unwrap(), &modifiers),
                    ),
                  ),
                  DirectiveArg::Dynamic(_) => ast.jsx_attribute_name_identifier(
                    Span::new(dir_start, dir_start + 1),
                    ast.atom(&format!(
                      "v-{}{}",
                      dir.name,
                      self.parse_argument(dir.argument.as_ref().unwrap(), &modifiers)
                    )),
                  ),
                }
              } else {
                ast.jsx_attribute_name_identifier(
                  dir.head_loc.span(),
                  self.ast.atom(&format!(
                    "v-{}{}",
                    dir.name,
                    Self::parse_modifiers(&modifiers)
                  )),
                )
              }
            }
          },
          if let Some(expr) = &dir.expression {
            // v-for="item of list" => v-for="item in list"
            let raw = if dir.name == "for" {
              ast.atom(&expr.content.raw.replace(" of ", "in")).as_str()
            } else {
              expr.content.raw
            };
            let mut expression = self.parse_expression(raw, expr.location.start.offset);

            if dir.name == "slot" || dir.name == "for" {
              DefaultValueToAssignment::new(self).visit_expression(&mut expression);
            }

            Some(ast.jsx_attribute_value_expression_container(
              Span::new(expr.location.start.offset as u32 + 1, dir_end - 1),
              self.parse_dynamic_argument(&dir, expression).into(),
            ))
          } else if let Some(argument) = &dir.argument
            && let DirectiveArg::Dynamic(_) = argument
          {
            // v-slot:[name]
            Some(
              ast.jsx_attribute_value_expression_container(
                SPAN,
                self
                  .parse_dynamic_argument(&dir, ast.expression_identifier(SPAN, "undefined"))
                  .into(),
              ),
            )
          } else {
            None
          },
        )
      }
    }
  }

  fn parse_dynamic_argument(
    &self,
    dir: &Directive<'a>,
    expression: Expression<'a>,
  ) -> Expression<'a> {
    let ast = &self.ast;
    let head_name = dir.head_loc.span().source_text(self.source_text);
    let dir_start = dir.location.start.offset;
    if let Some(argument) = &dir.argument
      && let DirectiveArg::Dynamic(argument_str) = argument
    {
      let dynamic_arg_expression = self.parse_expression(
        argument_str,
        if head_name.starts_with("v-") {
          dir_start + 2 + dir.name.len() + 1
        } else {
          dir_start + 1
        },
      );
      ast.expression_object(
        SPAN,
        ast.vec1(ast.object_property_kind_object_property(
          SPAN,
          PropertyKind::Init,
          dynamic_arg_expression.into(),
          expression,
          false,
          false,
          true,
        )),
      )
    } else {
      expression
    }
  }

  fn parse_argument(&self, argument: &DirectiveArg, modifiers: &[&'a str]) -> Atom<'a> {
    self.ast.atom(&format!(
      "{}{}",
      match argument {
        DirectiveArg::Static(arg) => arg,
        DirectiveArg::Dynamic(_) => "",
      },
      Self::parse_modifiers(modifiers)
    ))
  }
  fn parse_modifiers(modifiers: &[&str]) -> String {
    if modifiers.is_empty() {
      String::new()
    } else {
      format!("_{}", modifiers.join("_"))
    }
  }

  fn parse_text(&self, text: &TextNode<'a>) -> JSXChild<'a> {
    let raw = self.ast.atom(text.text[0].raw);
    self
      .ast
      .jsx_child_text(text.location.span(), raw, Some(raw))
  }

  fn parse_comment(&self, comment: &SourceNode<'a>) -> JSXChild<'a> {
    let ast = self.ast;
    let span = comment.location.span();
    self.comments.borrow_mut().push(Comment::new(
      span.start + 1,
      span.end - 1,
      if comment.source.contains('\n') {
        CommentKind::MultiLineBlock
      } else {
        CommentKind::SingleLineBlock
      },
    ));
    ast.jsx_child_expression_container(
      span,
      ast.jsx_expression_empty_expression(Span::new(span.start + 1, span.end - 1)),
    )
  }

  fn parse_interpolation(&self, introp: &SourceNode<'a>) -> JSXChild<'a> {
    let ast = self.ast;
    let span = Span::new(
      introp.location.start.offset as u32 + 1,
      introp.location.end.offset as u32 - 1,
    );
    ast.jsx_child_expression_container(
      span,
      self
        .parse_expression(introp.source, span.start as usize)
        .into(),
    )
  }

  /// # Panics
  ///
  /// Panics if the expression cannot be parsed or if the first statement is not an expression.
  pub fn parse_expression(&self, source: &'a str, start: usize) -> Expression<'a> {
    let ast = &self.ast;
    if is_simple_identifier(source) {
      return ast.expression_identifier(
        Span::new(start as u32 + 1, (start + source.len() + 1) as u32),
        source,
      );
    }

    let source_text = ast
      .atom(&format!(
        "/*{}*/({})",
        // Using comments instead of whitespace to improve performance.
        &self.empty_str[..start - 4],
        &source
      ))
      .as_str();
    let mut program = oxc_parser::Parser::new(self.allocator, source_text, self.source_type)
      .parse()
      .program;
    let Some(Statement::ExpressionStatement(stmt)) = program.body.get_mut(0) else {
      panic!("parse expression error")
    };
    let Expression::ParenthesizedExpression(expression) = &mut stmt.expression else {
      unreachable!()
    };
    expression.expression.take_in(self.allocator)
  }

  fn offset(&self, start: usize) -> usize {
    start
      + self.source_text[start..]
        .chars()
        .take_while(|c| c.is_whitespace())
        .count()
  }

  fn roffset(&self, end: usize) -> usize {
    end
      - self.source_text[..end]
        .chars()
        .rev()
        .take_while(|c| c.is_whitespace())
        .count()
  }
}

struct DefaultValueToAssignment<'a, 'ctx> {
  context: &'ctx ParserImpl<'a>,
}
impl<'a, 'ctx> DefaultValueToAssignment<'a, 'ctx> {
  pub const fn new(context: &'ctx ParserImpl<'a>) -> Self {
    Self { context }
  }
}
impl<'a> VisitMut<'a> for DefaultValueToAssignment<'a, '_> {
  fn visit_object_property(&mut self, it: &mut oxc_ast::ast::ObjectProperty<'a>) {
    if it.value.span().end != it.span.end {
      let ast = &self.context.ast;
      let value_start = it.value.span().start as usize;
      let source = &self.context.source_text[value_start..it.span.end as usize];
      let mut expression = oxc_parser::Parser::new(
        ast.allocator,
        ast
          .atom(&format!(
            "/*{}*/{}",
            &self.context.empty_str[..it.span.start as usize - 4],
            source
          ))
          .as_str(),
        self.context.source_type,
      )
      .parse_expression()
      .unwrap();
      if let Expression::AssignmentExpression(expr) = &mut expression {
        if let AssignmentTarget::AssignmentTargetIdentifier(id) = &mut expr.left {
          id.span = SPAN;
        }
        it.value = expression.take_in(ast.allocator);
      }
    }
  }
}
