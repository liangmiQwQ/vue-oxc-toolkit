use std::collections::HashSet;

use oxc_allocator::Vec as ArenaVec;
use oxc_ast::ast::{JSXChild, Statement};

use oxc_span::SourceType;
use vue_compiler_core::{
  parser::{ElemProp, Element},
  util::{find_prop, prop_finder},
};

use crate::parser::{
  ParserImpl, RetParse, RetParseExt, error, modules::Merge, parse::SourceLocatonSpan,
};

impl<'a> ParserImpl<'a> {
  pub fn parse_script(&mut self, node: Element<'a>) -> RetParse<Option<JSXChild<'a>>> {
    let mut source_types: HashSet<&str> = HashSet::new();

    let lang = find_prop(&node, "lang")
      .and_then(|p| match p.get_ref() {
        ElemProp::Attr(p) => p.value.as_ref().map(|value| value.content.raw),
        ElemProp::Dir(_) => None,
      })
      .unwrap_or("js");

    source_types.insert(lang);

    if source_types.len() > 1 {
      error::multiple_script_tags(&mut self.errors, &source_types);
      return RetParse::panic();
    }

    self.source_type = if lang.starts_with("js") {
      SourceType::jsx()
    } else if lang.starts_with("ts") {
      SourceType::tsx()
    } else {
      error::unexpected_script_lang(&mut self.errors, lang);
      return RetParse::panic();
    };

    if let Some(child) = node.children.first() {
      let span = child.get_location().span();
      let source = span.source_text(self.source_text);

      let Some((mut body, module_record)) = self.oxc_parse(
        source,
        // SAFETY: lang is validated above to be "js" or "ts" based extensions which are valid for from_extension
        SourceType::from_extension(lang).unwrap(),
        span.start as usize,
      ) else {
        return Ok(None);
      };

      // Deal with modules record there
      let is_setup = prop_finder(&node, "setup").allow_empty().find().is_some();

      if is_setup {
        // Only merge imports, as exports are not allowed in <script setup>
        self.module_record.merge_imports(module_record);

        // Split imports and other statements
        let mut imports: ArenaVec<Statement<'a>> = self.ast.vec();
        let mut statements: ArenaVec<Statement<'a>> = self.ast.vec();

        for statement in body {
          match statement {
            Statement::ImportDeclaration(_) => imports.push(statement),
            _ => statements.push(statement),
          }
        }

        // Append imports to self.statements (top level)
        imports.append(&mut self.statements);
        self.statements = imports;
        // Replace self.setup with the rest (inside function).
        self.setup = statements;
      } else {
        self.module_record.merge(module_record);
        // Append all statements, do not replace all as probably exist imports statements
        self.statements.append(&mut body);
      }
    }

    RetParse::success(Some(self.parse_element(node, Some(self.ast.vec())).0))
  }
}
