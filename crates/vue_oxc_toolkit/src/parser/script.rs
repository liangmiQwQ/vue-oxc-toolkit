use std::collections::HashSet;

use oxc_ast::ast::JSXChild;
use oxc_diagnostics::OxcDiagnostic;
use oxc_span::SourceType;
use vue_compiler_core::{
  parser::{ElemProp, Element},
  util::find_prop,
};

use crate::parser::{ParserImpl, modules::Merge, parse::SourceLocatonSpan};

impl<'a> ParserImpl<'a> {
  pub fn parse_script(&mut self, node: Element<'a>) -> Option<JSXChild<'a>> {
    let mut source_types: HashSet<&str> = HashSet::new();

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
      self.errors.push(OxcDiagnostic::error(format!("Unsupported script language: {lang}")));
      return None;
    };

    if let Some(child) = node.children.first() {
      let span = child.get_location().span();
      let source = span.source_text(self.source_text);

      let (mut body, module_record) = self.oxc_parse(
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
        self.setup.append(&mut body);
      } else {
        self.module_record.merge(module_record);
        self.statements.append(&mut body);
      }
    }

    self.parse_element(node, Some(self.ast.vec()))
  }
}
