#![deny(clippy::all)]

use oxc_ast::ast::CommentKind;
use oxc_span::{GetSpan, SourceType, Span};
use vue_oxlint_parser::{
  VueParseConfig, VueParser,
  ast::{
    VAttributeOrDirective, VComment, VDirectiveArgumentKind, VDirectiveExpression, VElement, VNode,
    VScriptKind, VText, VueSingleFileComponent,
  },
};

use napi_derive::napi;
use oxc_allocator::Allocator;
use oxc_parser::ParseOptions;
use serde_json::{Value, json};

#[napi(object)]
pub struct NativeRange {
  pub start: u32,
  pub end: u32,
}

#[napi(object)]
pub struct NativeComment {
  #[napi(ts_type = "'Line' | 'Block'")]
  pub r#type: String,
  pub value: String,
  pub start: u32,
  pub end: u32,
}

#[napi(object)]
pub struct NativeDiagnostic {
  pub message: String,
  pub start: u32,
  pub end: u32,
}

#[napi(object)]
pub struct NativeTemplateToken {
  pub r#type: String,
  pub value: String,
  pub start: u32,
  pub end: u32,
}

#[napi(object)]
pub struct NativeParseResult {
  pub ast_json: String,
  #[napi(ts_type = "'script' | 'module'")]
  pub source_type: String,
  pub template_tokens: Vec<NativeTemplateToken>,
  pub comments: Vec<NativeComment>,
  pub irregular_whitespaces: Vec<NativeRange>,
  pub errors: Vec<NativeDiagnostic>,
  pub panicked: bool,
}

#[napi]
#[must_use]
#[allow(clippy::needless_pass_by_value, reason = "N-API owns string arguments at the boundary.")]
pub fn native_parse(source: String) -> NativeParseResult {
  let allocator_a = Allocator::new();
  let allocator_b = Allocator::new();
  let ret = VueParser::new(
    &allocator_a,
    &allocator_b,
    &source,
    ParseOptions::default(),
    VueParseConfig { track_clean_spans: false },
  )
  .parse();

  let ast = program_json(&ret.sfc, &source);

  NativeParseResult {
    ast_json: ast.to_string(),
    source_type: source_type_kind(ret.sfc.source_type).to_string(),
    template_tokens: ret
      .template_tokens
      .iter()
      .map(|token| NativeTemplateToken {
        r#type: format!("{:?}", token.kind),
        value: token.span.source_text(&source).to_string(),
        start: token.span.start,
        end: token.span.end,
      })
      .collect(),
    comments: ret
      .sfc
      .script_comments
      .iter()
      .map(|comment| {
        let comment_data =
          comment_data(&source, comment.kind, comment.span.start, comment.span.end);
        NativeComment {
          r#type: match comment.kind {
            CommentKind::Line => "Line",
            CommentKind::SingleLineBlock | CommentKind::MultiLineBlock => "Block",
          }
          .to_string(),
          value: comment_data.value.to_string(),
          start: comment_data.start,
          end: comment_data.end,
        }
      })
      .collect(),
    irregular_whitespaces: ret
      .irregular_whitespaces
      .iter()
      .map(|span| NativeRange { start: span.start, end: span.end })
      .collect(),
    errors: diagnostics_to_native(&ret.errors),
    panicked: ret.panicked,
  }
}

struct CommentData<'a> {
  value: &'a str,
  start: u32,
  end: u32,
}

fn comment_data(source: &str, kind: CommentKind, start: u32, end: u32) -> CommentData<'_> {
  let start = start as usize;
  let end = end as usize;

  if kind == CommentKind::Line {
    let value_start = start + 2;
    let end = line_comment_end(source, value_start);

    return CommentData {
      value: source.get(value_start..end).unwrap_or_default(),
      start: start as u32,
      end: end as u32,
    };
  }

  CommentData {
    value: source.get(start..end).unwrap_or_default(),
    start: start as u32,
    end: end as u32,
  }
}

fn line_comment_end(source: &str, value_start: usize) -> usize {
  source[value_start..].find('\n').map_or(source.len(), |newline| value_start + newline)
}

fn diagnostics_to_native(errors: &[oxc_diagnostics::OxcDiagnostic]) -> Vec<NativeDiagnostic> {
  errors
    .iter()
    .map(|error| {
      let (start, end) =
        error.labels.as_ref().and_then(|labels| labels.first()).map_or((0, 0), |label| {
          let start = label.offset() as u32;
          let end = start + label.len() as u32;
          (start, end)
        });

      NativeDiagnostic { message: error.message.to_string(), start, end }
    })
    .collect()
}

fn source_type_kind(source_type: SourceType) -> &'static str {
  if source_type.is_module() { "module" } else { "script" }
}

fn program_json(sfc: &VueSingleFileComponent<'_, '_>, source: &str) -> Value {
  json!({
    "type": "Program",
    "sourceType": source_type_kind(sfc.source_type),
    "body": [],
    "comments": [],
    "tokens": [],
    "range": [0, source.len()],
    "templateBody": {
      "type": "VDocumentFragment",
      "range": [0, source.len()],
      "children": sfc.children.iter().map(|node| vnode_json(node, source)).collect::<Vec<_>>(),
    },
  })
}

fn vnode_json(node: &VNode<'_, '_>, source: &str) -> Value {
  match node {
    VNode::Element(element) => element_json(element, source),
    VNode::Text(text) => text_json(text),
    VNode::Comment(comment) => comment_json(comment),
    VNode::Interpolation(interpolation) => json!({
      "type": "VExpressionContainer",
      "range": range(interpolation.span),
      "expression": {
        "type": expression_type_name(interpolation.expression),
        "range": range(interpolation.expression.span()),
      },
    }),
    VNode::CData(cdata) => json!({
      "type": "VText",
      "value": cdata.value,
      "range": range(cdata.span),
    }),
  }
}

fn element_json(element: &VElement<'_, '_>, source: &str) -> Value {
  json!({
    "type": "VElement",
    "rawName": element.start_tag.name_span.source_text(source),
    "range": range(element.span),
    "startTag": {
      "type": "VStartTag",
      "range": range(element.start_tag.span),
      "selfClosing": element.start_tag.self_closing,
      "attributes": element.start_tag.attributes.iter().map(attribute_json).collect::<Vec<_>>(),
    },
    "endTag": element.end_tag.as_ref().map(|tag| json!({
      "type": "VEndTag",
      "range": range(tag.span),
    })),
    "children": element.children.iter().map(|node| vnode_json(node, source)).collect::<Vec<_>>(),
    "script": element.script.as_ref().map(|script| json!({
      "kind": match script.kind {
        VScriptKind::Script => "script",
        VScriptKind::Setup => "setup",
      },
      "range": range(script.body_span),
      "bodyLength": script.program.body.len(),
    })),
  })
}

fn attribute_json(attribute: &VAttributeOrDirective<'_, '_>) -> Value {
  match attribute {
    VAttributeOrDirective::Attribute(attribute) => json!({
      "type": "VAttribute",
      "key": {
        "type": "VIdentifier",
        "name": attribute.key.name,
        "range": range(attribute.key.span),
      },
      "value": attribute.value.as_ref().map(|value| json!({
        "type": "VLiteral",
        "value": value.value,
        "range": range(value.span),
      })),
      "range": range(attribute.span),
    }),
    VAttributeOrDirective::Directive(directive) => json!({
      "type": "VDirective",
      "key": {
        "name": directive.key.name.name,
        "argument": directive.key.argument.as_ref().map(|argument| json!({
          "raw": argument.raw,
          "kind": match argument.kind {
            VDirectiveArgumentKind::Static => "static",
            VDirectiveArgumentKind::Dynamic => "dynamic",
          },
          "expression": argument.expression.map(|expression| json!({
            "type": expression_type_name(expression),
            "range": range(expression.span()),
          })),
          "range": range(argument.span),
        })),
        "modifiers": directive.key.modifiers.iter().map(|modifier| json!({
          "name": modifier.name,
          "range": range(modifier.span),
        })).collect::<Vec<_>>(),
        "range": range(directive.key.span),
      },
      "value": directive.value.as_ref().map(|value| json!({
        "raw": value.raw,
        "range": range(value.span),
        "expression": directive_expression_json(&value.expression),
      })),
      "range": range(directive.span),
    }),
  }
}

fn directive_expression_json(expression: &VDirectiveExpression<'_, '_>) -> Value {
  match expression {
    VDirectiveExpression::Expression(expression) => json!({
      "type": expression_type_name(expression),
      "range": range(expression.span()),
    }),
    VDirectiveExpression::VFor(v_for) => json!({
      "type": "VForExpression",
      "left": {
        "type": "FormalParameters",
        "range": range(v_for.left.span),
        "count": v_for.left.items.len(),
      },
      "right": {
        "type": expression_type_name(v_for.right),
        "range": range(v_for.right.span()),
      },
    }),
    VDirectiveExpression::VSlot(v_slot) => json!({
      "type": "VSlotExpression",
      "params": {
        "type": "FormalParameters",
        "range": range(v_slot.params.span),
        "count": v_slot.params.items.len(),
      },
    }),
    VDirectiveExpression::VOn(v_on) => json!({
      "type": "VOnExpression",
      "bodyLength": v_on.statements.len(),
    }),
  }
}

fn text_json(text: &VText<'_>) -> Value {
  json!({
    "type": "VText",
    "value": text.value,
    "range": range(text.span),
  })
}

fn comment_json(comment: &VComment<'_>) -> Value {
  json!({
    "type": "VComment",
    "value": comment.value,
    "range": range(comment.span),
  })
}

const fn expression_type_name(expression: &oxc_ast::ast::Expression<'_>) -> &'static str {
  match expression {
    oxc_ast::ast::Expression::BinaryExpression(_) => "BinaryExpression",
    oxc_ast::ast::Expression::Identifier(_) => "Identifier",
    oxc_ast::ast::Expression::StringLiteral(_) | oxc_ast::ast::Expression::NumericLiteral(_) => {
      "Literal"
    }
    _ => "Expression",
  }
}

const fn range(span: Span) -> [u32; 2] {
  [span.start, span.end]
}
