use oxc_allocator::{Allocator, CloneIn, TakeIn, Vec as ArenaVec};
use oxc_ast::{
  Comment, CommentKind, NONE,
  ast::{Expression, JSXAttributeItem, JSXChild, JSXExpression, PropertyKind, Statement},
};
use oxc_span::{GetSpanMut, SPAN, Span};
use vize_armature::{
  CommentNode, DirectiveNode, ElementNode, ElementType, InterpolationNode, PropNode,
  TemplateChildNode, TextNode,
};

use crate::{
  is_void_tag,
  parser::{
    ParserImpl,
    elements::{
      v_for::VForWrapper,
      v_if::{VIf, VIfManager},
      v_slot::VSlotWrapper,
    },
    error,
    parse::SourceLocatonSpan,
  },
};

mod directive;
mod v_for;
mod v_if;
mod v_slot;

impl<'a: 'b, 'b> ParserImpl<'a> {
  fn parse_children(
    &mut self,
    start: u32,
    end: u32,
    children: &[TemplateChildNode<'_>],
  ) -> ArenaVec<'a, JSXChild<'a>> {
    let ast = self.ast;
    if children.is_empty() {
      return ast.vec();
    }
    let mut result = self.ast.vec_with_capacity(children.len() + 2);

    // Process the whitespaces text there <div>____<br>_____</div>
    if let Some(first) = children.first()
      && matches!(first, TemplateChildNode::Element(_) | TemplateChildNode::Interpolation(_))
      && start != first.loc().start.offset
    {
      let span = Span::new(start, first.loc().start.offset);
      let value = span.source_text(self.source_text);
      result.push(ast.jsx_child_text(span, value, Some(ast.str(value))));
    }

    let last = if let Some(last) = children.last()
      && matches!(last, TemplateChildNode::Element(_) | TemplateChildNode::Interpolation(_))
      && end != last.loc().end.offset
    {
      let span = Span::new(last.loc().end.offset, end);
      let value = span.source_text(self.source_text);
      Some(ast.jsx_child_text(span, value, Some(ast.str(value))))
    } else {
      None
    };

    let mut v_if_manager = VIfManager::new(&ast);
    for child in children {
      match child {
        TemplateChildNode::Element(node) => {
          let (child, v_if) = self.parse_element_ref(node, None);

          if let Some(v_if) = v_if {
            if let Some(child) = self.add_v_if(child, v_if, &mut v_if_manager) {
              result.push(child);
            }
          } else {
            if let Some(chain) = v_if_manager.take_chain() {
              result.push(chain);
            }
            result.push(child);
          }
        }
        TemplateChildNode::Text(text) => result.push(self.parse_text(text)),
        TemplateChildNode::Comment(comment) => result.push(self.parse_comment(comment)),
        TemplateChildNode::Interpolation(interp) => {
          result.push(self.parse_interpolation(interp));
        }
        _ => {
          // Other node types (If, For, TextCall, etc.) should not appear at parse stage
        }
      }
    }

    if let Some(chain) = v_if_manager.take_chain() {
      result.push(chain);
    }
    if let Some(last) = last {
      result.push(last);
    }

    result
  }

  pub fn parse_element_ref(
    &mut self,
    node: &ElementNode<'_>,
    children: Option<ArenaVec<'a, JSXChild<'a>>>,
  ) -> (JSXChild<'a>, Option<VIf<'a>>) {
    let ast = self.ast;

    let tag_src = node.loc.span().source_text(self.source_text);
    // Extract just the tag name from the source (between < and first whitespace or >)
    let tag_name_str = tag_src[1..] // skip '<'
      .split(|c: char| c.is_whitespace() || c == '>' || c == '/')
      .next()
      .unwrap_or("");

    let open_element_span = {
      let start = node.loc.start.offset;
      let tag_name_end = if let Some(prop) = node.props.last() {
        prop.loc().end.offset
      } else {
        start + 1 /* < */ + tag_name_str.len() as u32
      };
      let end = memchr::memchr(b'>', &self.source_text.as_bytes()[tag_name_end as usize..])
        .map(|i| tag_name_end + i as u32 + 1)
        .unwrap(); // SAFETY: The tag must be closed. Or parser will treat it as panicked.
      Span::new(start, end)
    };

    let location_span = node.loc.span();
    let end_element_span = {
      if location_span.source_text(self.source_text).ends_with("/>") || is_void_tag!(tag_name_str) {
        node.loc.span()
      } else {
        let end = node.loc.end.offset;
        let start = memchr::memrchr(b'<', &self.source_text.as_bytes()[..end as usize])
          .map(|i| i as u32)
          .unwrap();
        Span::new(start, end)
      }
    };

    // Use different JSXElementName for component and normal element
    let allocator = Allocator::new();
    let mut element_name = {
      let name_span = Span::sized(open_element_span.start + 1, tag_name_str.len() as u32);

      if tag_name_str.contains('.')
        && let Some(expr) = unsafe {
          let original_source_type = self.source_type;
          self.source_type = self.source_type.with_jsx(true);

          let expr = self.parse_expression(name_span, b"(<", b"/>)", &allocator);

          self.source_type = original_source_type;

          expr
        }
        && let Expression::JSXElement(mut jsx_element) = expr
      {
        jsx_element.opening_element.name.take_in(self.allocator)
      } else if tag_name_str.contains('-') {
        let name = tag_name_str
          .split('-')
          .map(|s| {
            let mut bytes = s.as_bytes().to_vec();
            bytes[0] = bytes[0].to_ascii_uppercase();
            String::from_utf8(bytes).unwrap()
          })
          .collect::<String>();

        ast.jsx_element_name_identifier_reference(name_span, ast.str(&name))
      } else {
        let name = ast.str(tag_name_str);
        if node.tag_type == ElementType::Component {
          ast.jsx_element_name_identifier_reference(name_span, name)
        } else {
          ast.jsx_element_name_identifier(name_span, name)
        }
      }
    }
    .clone_in(self.allocator);

    let mut v_for_wrapper = VForWrapper::new(&ast);
    let mut v_slot_wrapper = VSlotWrapper::new(&ast);
    let mut v_if_state: Option<VIf<'a>> = None;
    let mut attributes = ast.vec();
    for prop in &node.props {
      attributes.push(self.parse_prop(
        prop,
        &mut v_for_wrapper,
        &mut v_slot_wrapper,
        &mut v_if_state,
      ));
    }

    let children = match children {
      Some(children) => children,
      None => v_slot_wrapper.wrap(self.parse_children(
        open_element_span.end,
        end_element_span.start,
        &node.children,
      )),
    };

    let opening_element_name = element_name.clone_in(self.allocator);

    let closing_element = if location_span.source_text(self.source_text).ends_with("/>") {
      Some(ast.jsx_closing_element(SPAN, ast.jsx_element_name_identifier(SPAN, ast.str(""))))
    } else if is_void_tag!(tag_name_str) {
      None
    } else {
      Some(ast.jsx_closing_element(end_element_span, {
        let span = Span::sized(end_element_span.start + 2, tag_name_str.len() as u32);
        *element_name.span_mut() = span;
        element_name
      }))
    };

    (
      v_for_wrapper.wrap(ast.jsx_element(
        location_span,
        ast.jsx_opening_element(open_element_span, opening_element_name, NONE, attributes),
        children,
        closing_element,
      )),
      v_if_state,
    )
  }

  fn parse_prop(
    &mut self,
    prop: &PropNode<'_>,
    v_for_wrapper: &mut VForWrapper<'_, 'a>,
    v_slot_wrapper: &mut VSlotWrapper<'_, 'a>,
    v_if_state: &mut Option<VIf<'a>>,
  ) -> JSXAttributeItem<'a> {
    let ast = self.ast;
    match prop {
      // For normal attributes, like <div class="w-100" />
      PropNode::Attribute(attr) => {
        let attr_end = self.roffset(attr.loc.end.offset as usize) as u32;
        let attr_span = Span::new(attr.loc.start.offset, attr_end);
        ast.jsx_attribute_item_attribute(
          attr_span,
          ast.jsx_attribute_name_identifier(attr.name_loc.span(), {
            let name_text = attr.name_loc.span().source_text(self.source_text);
            ast.str(name_text)
          }),
          if let Some(value) = &attr.value {
            // vize TextNode.loc doesn't include quotes, so use it directly for content
            let value_span = value.loc.span();
            Some(ast.jsx_attribute_value_string_literal(
              value_span,
              ast.str(value_span.source_text(self.source_text)),
              None,
            ))
          } else {
            None
          },
        )
      }
      // Directive, starts with `v-`
      PropNode::Directive(dir) => {
        let dir_start = dir.loc.start.offset;
        let dir_end = self.roffset(dir.loc.end.offset as usize) as u32;

        let dir_name = self.parse_directive_name(dir);
        // Analyze v-slot and v-for, no matter whether there is an expression
        if dir.name.as_str() == "slot" {
          self.analyze_v_slot(dir, v_slot_wrapper, &dir_name);
        } else if dir.name.as_str() == "for" {
          self.analyze_v_for(dir, v_for_wrapper);
        } else if dir.name.as_str() == "else" {
          // v-else can have no expression
          *v_if_state = Some(VIf::Else);
        }

        if matches!(dir.name.as_str(), "if" | "else-if") && dir.exp.is_none() {
          error::v_if_else_without_expression(&mut self.errors, dir.loc.span());
        }

        let value = if let Some(exp) = &dir.exp {
          let exp_loc = exp.loc();
          // vize expression loc doesn't include quotes
          let expr_span = exp_loc.span();
          Some(
            ast.jsx_attribute_value_expression_container(
              // -1 to include the opening quote in the container span
              Span::new(expr_span.start.saturating_sub(1), dir_end),
              ((|| {
                // Use placeholder for v-for and v-slot
                if matches!(dir.name.as_str(), "for" | "slot" | "else") {
                  None
                } else {
                  let expr = self.parse_pure_expression(expr_span);
                  if dir.name.as_str() == "if" {
                    *v_if_state = expr.map(VIf::If);
                    None
                  } else if dir.name.as_str() == "else-if" {
                    *v_if_state = expr.map(VIf::ElseIf);
                    None
                  } else {
                    Some(JSXExpression::from(self.parse_dynamic_argument(dir, expr?)?))
                  }
                }
              })())
              .unwrap_or_else(|| JSXExpression::EmptyExpression(ast.jsx_empty_expression(SPAN))),
            ),
          )
        } else if let Some(arg) = &dir.arg
          && !is_static_arg(arg)
          && let Some(argument) =
            self.parse_dynamic_argument(dir, ast.expression_identifier(SPAN, "undefined"))
        {
          // v-slot:[name]
          Some(ast.jsx_attribute_value_expression_container(SPAN, argument.into()))
        } else {
          None
        };

        ast.jsx_attribute_item_attribute(
          Span::new(dir_start, dir_end),
          // Attribute Name
          dir_name,
          // Attribute Value
          value,
        )
      }
    }
  }

  fn parse_dynamic_argument(
    &mut self,
    dir: &DirectiveNode<'_>,
    expression: Expression<'a>,
  ) -> Option<Expression<'a>> {
    let head_span = self.compute_head_span(dir);
    let head_name = head_span.source_text(self.source_text);
    let dir_start = dir.loc.start.offset;
    if let Some(arg) = &dir.arg
      && !is_static_arg(arg)
    {
      let arg_loc = arg.loc();
      let dynamic_arg_expression = self.parse_pure_expression({
        Span::sized(
          if head_name.starts_with("v-") {
            dir_start + 2 + dir.name.len() as u32 + 2 // v-bind:[arg] -> skip `:[` (2 chars)
          } else {
            dir_start + 2 // :[arg] -> skip `:[` (2 chars)
          },
          (arg_loc.end.offset - arg_loc.start.offset) as u32,
        )
      })?;

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

  fn parse_text(&self, text: &TextNode) -> JSXChild<'a> {
    let span = text.loc.span();
    let raw = self.ast.str(span.source_text(self.source_text));
    self.ast.jsx_child_text(span, raw, Some(raw))
  }

  fn parse_comment(&mut self, comment: &CommentNode) -> JSXChild<'a> {
    let ast = self.ast;
    let span = comment.loc.span();
    let content = span.source_text(self.source_text);
    self.comments.push(Comment::new(
      span.start + 1,
      span.end - 1,
      if content.contains('\n') {
        CommentKind::MultiLineBlock
      } else {
        CommentKind::SingleLineBlock
      },
    ));
    ast.jsx_child_expression_container(span, ast.jsx_expression_empty_expression(SPAN))
  }

  fn parse_interpolation(&mut self, introp: &InterpolationNode<'_>) -> JSXChild<'a> {
    let ast = self.ast;
    let container_span = introp.loc.span();
    // vize InterpolationNode.content.loc() gives the expression span (without {{ }})
    let expr_span = introp.content.loc().span();

    ast.jsx_child_expression_container(
      container_span,
      self
        .parse_pure_expression(expr_span)
        .map_or_else(|| ast.jsx_expression_empty_expression(SPAN), JSXExpression::from),
    )
  }

  pub fn parse_pure_expression(&mut self, span: Span) -> Option<Expression<'a>> {
    let allocator = Allocator::new();
    // SAFETY: use `()` as wrap
    unsafe { self.parse_expression(span, b"(", b")", &allocator).clone_in(self.allocator) }
  }

  /// Parse expression with [`oxc_parser`]
  ///
  /// ## Safety
  /// - `start_wrap` must start with `(`
  /// - `end_wrap` must end with `)`
  pub unsafe fn parse_expression(
    &mut self,
    span: Span,
    start_wrap: &[u8],
    end_wrap: &[u8],
    allocator: &'b Allocator,
  ) -> Option<Expression<'b>> {
    let (_, mut body, _) = self.oxc_parse(span, start_wrap, end_wrap, Some(allocator))?;

    let Some(Statement::ExpressionStatement(stmt)) = body.get_mut(0) else { unreachable!() };
    let Expression::ParenthesizedExpression(expression) = &mut stmt.expression else {
      unreachable!()
    };
    Some(expression.expression.take_in(self.allocator))
  }

  fn roffset(&self, end: usize) -> usize {
    end - self.source_text[..end].chars().rev().take_while(|c| c.is_whitespace()).count()
  }

  /// Compute the "head" span of a directive — the directive prefix + name + argument portion
  /// before the `=` sign or end of directive if no value.
  fn compute_head_span(&self, dir: &DirectiveNode<'_>) -> Span {
    let dir_text = dir.loc.span().source_text(self.source_text);
    let head_end =
      dir_text.find('=').map(|i| dir.loc.start.offset + i as u32).unwrap_or(dir.loc.end.offset);
    Span::new(dir.loc.start.offset, head_end)
  }
}

/// Check if a directive argument expression is static
fn is_static_arg(arg: &vize_armature::ExpressionNode<'_>) -> bool {
  match arg {
    vize_armature::ExpressionNode::Simple(s) => s.is_static,
    vize_armature::ExpressionNode::Compound(_) => false,
  }
}
