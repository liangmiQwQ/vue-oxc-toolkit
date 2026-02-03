use oxc_allocator::{TakeIn, Vec as ArenaVec};
use oxc_ast::{
  Comment, CommentKind, NONE,
  ast::{Expression, JSXAttributeItem, JSXChild, JSXExpression, PropertyKind, Statement},
};
use oxc_span::{SPAN, Span};
use vue_compiler_core::parser::{
  AstNode, Directive, DirectiveArg, ElemProp, Element, SourceNode, TextNode,
};

use crate::{
  is_void_tag,
  parser::{
    ParserImpl,
    elements::{v_for::VForWrapper, v_slot::VSlotWrapper},
    parse::SourceLocatonSpan,
  },
};

mod directive;
mod v_for;
mod v_slot;

impl<'a> ParserImpl<'a> {
  fn parse_children(
    &mut self,
    start: u32,
    end: u32,
    children: Vec<AstNode<'a>>,
  ) -> ArenaVec<'a, JSXChild<'a>> {
    let ast = self.ast;
    if children.is_empty() {
      return ast.vec();
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
        AstNode::Element(node) => self.parse_element(node, None),
        AstNode::Text(text) => self.parse_text(&text),
        AstNode::Comment(comment) => self.parse_comment(&comment),
        AstNode::Interpolation(interp) => self.parse_interpolation(&interp),
      });
    }

    if let Some(last) = last {
      result.push(last);
    }

    result
  }

  pub fn parse_element(
    &mut self,
    node: Element<'a>,
    children: Option<ArenaVec<'a, JSXChild<'a>>>,
  ) -> JSXChild<'a> {
    let ast = self.ast;

    let open_element_span = {
      let start = node.location.start.offset;
      let tag_name_end = if let Some(prop) = node.properties.last() {
        match prop {
          ElemProp::Attr(prop) => prop.location.end.offset,
          ElemProp::Dir(prop) => prop.location.end.offset,
        }
      } else {
        start + 1 /* < */ + node.tag_name.len()
      };
      let end = memchr::memchr(b'>', &self.source_text.as_bytes()[tag_name_end..])
        .map(|i| tag_name_end + i + 1)
        .unwrap(); // SAFETY: The tag must be closed. Or parser will treat it as panicked.
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
      if let Some(attr) = self.parse_attribute(prop, &mut v_for_wrapper, &mut v_slot_wrapper) {
        attributes.push(attr);
      }
    }

    let children = match children {
      Some(children) => children,
      None => v_slot_wrapper.wrap(self.parse_children(
        open_element_span.end,
        end_element_span.start,
        node.children,
      )),
    };

    v_for_wrapper.wrap(ast.jsx_element(
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
    ))
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

        let dir_name = self.parse_directive_name(&dir);
        // Analyze v-slot and v-for, no matter whether there is an expression
        if dir.name == "slot" {
          self.analyze_v_slot(&dir, v_slot_wrapper, &dir_name)?;
        } else if dir.name == "for" {
          self.analyze_v_for(&dir, v_for_wrapper)?;
        }
        let attribute_value = if let Some(expr) = &dir.expression {
          Some(ast.jsx_attribute_value_expression_container(
            Span::new(expr.location.start.offset as u32 + 1, dir_end - 1),
            // Use placeholder for v-for and v-slot
            if matches!(dir.name, "for" | "slot") {
              JSXExpression::EmptyExpression(ast.jsx_empty_expression(SPAN))
            } else {
              // For possible dynamic arguments
              let expr = self.parse_expression(expr.content.raw, expr.location.start.offset + 1)?; // +1 to skip the opening quote
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
          dir_start + 2 + dir.name.len() + 2 // v-bind:[arg] -> skip `:[` (2 chars)
        } else {
          dir_start + 2 // :[arg] -> skip `:[` (2 chars)
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
    ast.jsx_child_expression_container(span, ast.jsx_expression_empty_expression(SPAN))
  }

  fn parse_interpolation(&mut self, introp: &SourceNode<'a>) -> JSXChild<'a> {
    let ast = self.ast;
    // Use full span for container (includes {{ and }})
    let container_span = introp.location.span();
    // Expression starts after {{ (2 characters)
    let expr_start = introp.location.start.offset + 2;

    ast.jsx_child_expression_container(
      container_span,
      match self.parse_expression(introp.source, expr_start) {
        Some(expr) => JSXExpression::from(expr),
        None => ast.jsx_expression_empty_expression(SPAN),
      },
    )
  }

  pub fn parse_expression(&mut self, source: &'a str, start: usize) -> Option<Expression<'a>> {
    let (mut body, _) =
      self.oxc_parse(&format!("({source})"), self.source_type, start.saturating_sub(1))?;

    let Some(Statement::ExpressionStatement(stmt)) = body.get_mut(0) else {
      // SAFETY: We always wrap the source in parentheses, so it should always be an expression statement
      // if it was valid partially. If it's invalid, the parser might return empty body if it fails early.
      unreachable!()
    };
    let Expression::ParenthesizedExpression(expression) = &mut stmt.expression else {
      unreachable!()
    };
    Some(expression.expression.take_in(self.allocator))
  }

  fn roffset(&self, end: usize) -> usize {
    end - self.source_text[..end].chars().rev().take_while(|c| c.is_whitespace()).count()
  }
}
