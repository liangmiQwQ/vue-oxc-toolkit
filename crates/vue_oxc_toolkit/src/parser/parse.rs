use std::cell::RefCell;
use std::collections::HashSet;

use oxc_allocator::{self, Dummy, TakeIn, Vec as ArenaVec};
use oxc_ast::ast::{
  Expression, FormalParameterKind, JSXAttributeItem, JSXChild, JSXExpression, Program,
  PropertyKind, Statement,
};
use oxc_ast::{Comment, CommentKind, NONE};
use oxc_diagnostics::OxcDiagnostic;
use oxc_span::{SPAN, SourceType, Span};
use oxc_syntax::module_record::ModuleRecord;
use vue_compiler_core::SourceLocation;
use vue_compiler_core::parser::{
  AstNode, Directive, DirectiveArg, ElemProp, Element, ParseOption, Parser, SourceNode, TextNode,
  WhitespaceStrategy,
};
use vue_compiler_core::scanner::{ScanOption, Scanner};
use vue_compiler_core::util::find_prop;

use crate::parser::directive::v_for::VForWrapper;
use crate::parser::directive::v_slot::VSlotWrapper;
use crate::parser::error::OxcErrorHandler;
use crate::parser::modules::Merge;

use super::ParserImpl;
use super::ParserImplReturn;

pub trait SourceLocatonSpan {
  fn span(&self) -> Span;
}

impl SourceLocatonSpan for SourceLocation {
  fn span(&self) -> Span {
    Span::new(self.start.offset as u32, self.end.offset as u32)
  }
}

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

impl<'a> ParserImpl<'a> {
  fn oxc_parse(
    &mut self,
    source: &str,
    source_type: SourceType,
    start: usize,
  ) -> Option<(ArenaVec<'a, Statement<'a>>, ModuleRecord<'a>)> {
    let source_text = self.ast.atom(&self.pad_source(source, start));
    let mut ret = oxc_parser::Parser::new(self.allocator, source_text.as_str(), source_type)
      .with_options(self.options)
      .parse();

    self.errors.append(&mut ret.errors);
    if ret.panicked {
      // TODO: do not panic for js parsing error
      None
    } else {
      self.comments.extend(&ret.program.comments[1..]);
      Some((ret.program.body, ret.module_record))
    }
  }

  /// A workaround
  /// Use comment placeholder to make the location AST returned correct
  /// The start must > 4 in any valid Vue files
  fn pad_source(&self, source: &str, start: usize) -> String {
    format!("/*{}*/{source}", &self.empty_str[..start - 4])
  }
}

impl<'a> ParserImpl<'a> {
  pub fn parse(mut self) -> ParserImplReturn<'a> {
    match self.get_root_children() {
      Some(children) => {
        let span = Span::new(0, self.source_text.len() as u32);
        self.fix_module_records(span);

        ParserImplReturn {
          program: self.ast.program(
            span,
            self.source_type,
            self.source_text,
            self.comments.take_in(self.ast.allocator),
            None, // no hashbang needed for vue files
            self.ast.vec(),
            self.ast.vec1(self.ast.statement_expression(
              SPAN,
              self.ast.expression_jsx_fragment(
                SPAN,
                self.ast.jsx_opening_fragment(SPAN),
                children,
                self.ast.jsx_closing_fragment(SPAN),
              ),
            )),
          ),
          fatal: false,
          errors: self.errors,
          module_record: self.module_record,
        }
      }
      None => ParserImplReturn {
        program: Program::dummy(self.allocator),
        fatal: true,
        errors: self.errors,
        module_record: ModuleRecord::new(self.allocator),
      },
    }
  }

  fn get_root_children(&mut self) -> Option<ArenaVec<'a, JSXChild<'a>>> {
    let parser = Parser::new(ParseOption {
      whitespace: WhitespaceStrategy::Preserve,
      is_void_tag: |name| is_void_tag!(name),
      ..Default::default()
    });

    // get ast from vue-compiler-core
    let scanner = Scanner::new(ScanOption::default());
    // error processing
    let errors = RefCell::from(&mut self.errors);
    let panicked = RefCell::from(false);
    let tokens = scanner.scan(self.source_text, OxcErrorHandler::new(&errors, &panicked));
    let result = parser.parse(tokens, OxcErrorHandler::new(&errors, &panicked));

    if *panicked.borrow() {
      return None;
    }

    let mut source_types: HashSet<&str> = HashSet::new();
    let mut children = self.ast.vec();
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
              self.errors.push(OxcDiagnostic::error(format!(
                "Multiple script tags with different languages: {source_types:?}"
              )));

              return None;
            }

            self.source_type = if lang.starts_with("js") {
              SourceType::jsx()
            } else if lang.starts_with("ts") {
              SourceType::tsx()
            } else {
              self
                .errors
                .push(OxcDiagnostic::error(format!("Unsupported script language: {lang}")));

              return None;
            };

            let script_block = if let Some(child) = node.children.first() {
              let span = child.get_location().span();
              let source = span.source_text(self.source_text);

              let (body, module_record) = self.oxc_parse(
                source,
                // SAFETY: lang is validated above to be "js" or "ts" based extensions which are valid for from_extension
                SourceType::from_extension(lang).unwrap(),
                span.start as usize,
              )?;

              // Deal with modules record there
              let is_setup = find_prop(&node, "setup").is_some();

              if is_setup {
                // Only merge imports, as exports are not allowed in <script setup>
                self.module_record.merge_imports(module_record);
              } else {
                self.module_record.merge(module_record);
              }

              body
            } else {
              self.ast.vec()
            };
            children.push(self.parse_element(
              node,
              Some(self.ast.vec1(self.ast.jsx_child_expression_container(
                SPAN,
                JSXExpression::ArrowFunctionExpression(self.ast.alloc_arrow_function_expression(
                  SPAN,
                  false,
                  false,
                  NONE,
                  self.ast.formal_parameters(
                    SPAN,
                    FormalParameterKind::ArrowFormalParameters,
                    self.ast.vec(),
                    NONE,
                  ),
                  NONE,
                  self.ast.function_body(SPAN, self.ast.vec(), script_block),
                )),
              ))),
            )?);
          } else if node.tag_name == "template" {
            children.push(self.parse_element(node, None)?);
          }
        }
        AstNode::Text(text) => children.push(self.parse_text(&text)),
        AstNode::Comment(comment) => children.push(self.parse_comment(&comment)),
        AstNode::Interpolation(interp) => children.push(self.parse_interpolation(&interp)?),
      }
    }

    Some(children)
  }

  fn parse_children(
    &mut self,
    start: u32,
    end: u32,
    children: Vec<AstNode<'a>>,
  ) -> Option<ArenaVec<'a, JSXChild<'a>>> {
    let ast = self.ast;
    if children.is_empty() {
      return Some(ast.vec());
    }
    let mut result = self.ast.vec_with_capacity(children.len() + 2);

    // Process the whitespaces text there <div>____<br>_____</div>
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

    for child in children {
      result.push(match child {
        AstNode::Element(node) => self.parse_element(node, None)?,
        AstNode::Text(text) => self.parse_text(&text),
        AstNode::Comment(comment) => self.parse_comment(&comment),
        AstNode::Interpolation(interp) => self.parse_interpolation(&interp)?,
      });
    }

    if let Some(last) = last {
      result.push(last);
    }

    Some(result)
  }

  fn parse_element(
    &mut self,
    node: Element<'a>,
    children: Option<ArenaVec<'a, JSXChild<'a>>>,
  ) -> Option<JSXChild<'a>> {
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
    let tag_name = node.tag_name;
    let end_element_span = {
      if location_span.source_text(self.source_text).ends_with("/>") || is_void_tag!(tag_name) {
        node.location.span()
      } else {
        let end = node.location.end.offset;
        let start = self.roffset(end).saturating_sub(tag_name.len() + 3) as u32;
        Span::new(start, end as u32)
      }
    };

    let mut v_for_wrapper = VForWrapper::new(&ast);
    let mut v_slot_wrapper = VSlotWrapper::new(&ast);
    let mut attributes = ast.vec();
    for prop in node.properties {
      attributes.push(self.parse_attribute(prop, &mut v_for_wrapper, &mut v_slot_wrapper)?);
    }

    let children = match children {
      Some(children) => children,
      None => v_slot_wrapper.wrap(self.parse_children(
        open_element_span.end,
        end_element_span.start,
        node.children,
      )?),
    };

    Some(v_for_wrapper.wrap(ast.jsx_element(
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
        attributes,
      ),
      children,
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
    )))
  }

  fn parse_attribute(
    &mut self,
    prop: ElemProp<'a>,
    v_for_wrapper: &mut VForWrapper<'_, 'a>,
    v_slot_wrapper: &mut VSlotWrapper<'_, 'a>,
  ) -> Option<JSXAttributeItem<'a>> {
    let ast = self.ast;
    match prop {
      // For normal attributes, like <div class="w-100" />
      ElemProp::Attr(attr) => {
        let attr_end = self.roffset(attr.location.end.offset) as u32;
        let attr_span = Span::new(attr.location.start.offset as u32, attr_end);
        Some(ast.jsx_attribute_item_attribute(
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
        ))
      }
      // Directive, starts with `v-`
      ElemProp::Dir(dir) => {
        let dir_start = dir.location.start.offset as u32;
        let dir_end = self.roffset(dir.location.end.offset) as u32;

        let dir_name = self.parse_directive_name(&dir)?;
        let attribute_value = if let Some(expr) = &dir.expression {
          Some(ast.jsx_attribute_value_expression_container(
            Span::new(expr.location.start.offset as u32 + 1, dir_end - 1),
            // Use placeholder for v-for and v-slot
            if dir.name == "for" {
              self.analyze_v_for(&dir, v_for_wrapper)?;
              JSXExpression::EmptyExpression(ast.jsx_empty_expression(SPAN))
            } else if dir.name == "slot" {
              self.analyze_v_slot(&dir, v_slot_wrapper, &dir_name)?;
              JSXExpression::EmptyExpression(ast.jsx_empty_expression(SPAN))
            } else {
              // For possible dynamic arguments
              let expr = self.parse_expression(expr.content.raw, expr.location.start.offset)?;
              self.parse_dynamic_argument(&dir, expr)?.into()
            },
          ))
        } else if let Some(argument) = &dir.argument
          && let DirectiveArg::Dynamic(_) = argument
        {
          // v-slot:[name]
          Some(ast.jsx_attribute_value_expression_container(
            SPAN,
            self.parse_dynamic_argument(&dir, ast.expression_identifier(SPAN, "undefined"))?.into(),
          ))
        } else {
          None
        };

        Some(ast.jsx_attribute_item_attribute(
          Span::new(dir_start, dir_end),
          // Attribute Name
          dir_name,
          // Attribute Value
          attribute_value,
        ))
      }
    }
  }

  fn parse_dynamic_argument(
    &mut self,
    dir: &Directive<'a>,
    expression: Expression<'a>,
  ) -> Option<Expression<'a>> {
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
      )?;
      Some(self.ast.expression_object(
        SPAN,
        self.ast.vec1(self.ast.object_property_kind_object_property(
          SPAN,
          PropertyKind::Init,
          dynamic_arg_expression.into(),
          expression,
          false,
          false,
          true,
        )),
      ))
    } else {
      Some(expression)
    }
  }

  fn parse_text(&self, text: &TextNode<'a>) -> JSXChild<'a> {
    let raw = self.ast.atom(&text.text.iter().map(|t| t.raw).collect::<String>());
    self.ast.jsx_child_text(text.location.span(), raw, Some(raw))
  }

  fn parse_comment(&mut self, comment: &SourceNode<'a>) -> JSXChild<'a> {
    let ast = self.ast;
    let span = comment.location.span();
    self.comments.push(Comment::new(
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

  fn parse_interpolation(&mut self, introp: &SourceNode<'a>) -> Option<JSXChild<'a>> {
    let ast = self.ast;
    let span =
      Span::new(introp.location.start.offset as u32 + 1, introp.location.end.offset as u32 - 1);
    Some(ast.jsx_child_expression_container(
      span,
      self.parse_expression(introp.source, span.start as usize)?.into(),
    ))
  }

  pub fn parse_expression(&mut self, source: &'a str, start: usize) -> Option<Expression<'a>> {
    let (mut body, _) =
      self.oxc_parse(&format!("({source})"), self.source_type, start.saturating_sub(1))?;

    let Some(Statement::ExpressionStatement(stmt)) = body.get_mut(0) else {
      // SAFETY: We always wrap the source in parentheses, so it should always be an expression statement
      // if it was valid partially. If it's invalid, the parser might return empty body if it fails early.
      // unreachable!()
      return None;
    };
    let Expression::ParenthesizedExpression(expression) = &mut stmt.expression else {
      // unreachable!()
      return None;
    };
    Some(expression.expression.take_in(self.allocator))
  }

  fn offset(&self, start: usize) -> usize {
    start + self.source_text[start..].chars().take_while(|c| c.is_whitespace()).count()
  }

  fn roffset(&self, end: usize) -> usize {
    end - self.source_text[..end].chars().rev().take_while(|c| c.is_whitespace()).count()
  }
}

#[cfg(test)]
mod tests {
  use crate::test_ast;

  #[test]
  fn basic_vue() {
    test_ast!("basic.vue");
    test_ast!("typescript.vue");
    test_ast!("void.vue");
  }

  #[test]
  fn comments() {
    test_ast!("comments.vue");
  }

  #[test]
  fn errors() {
    test_ast!("error/template.vue", true, true);
    test_ast!("error/interpolation.vue", true, true);
    test_ast!("error/recoverable-script.vue", true, false);
    test_ast!("error/recoverable-directive.vue", true, false);
    test_ast!("error/irrecoverable-script.vue", true, true);
    test_ast!("error/irrecoverable-directive.vue", true, true);
  }
}
