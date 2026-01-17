# Vue OXC Semantic

A toolkit to parse `.vue` file into semantically correct oxc jsx/tsx ast. Include generate `module_record`, `irregular_whitespaces` which are required by oxc_linter.

## Usage

```rust
use vue_oxc_toolkit::VueOxcParser;
use oxc_allocator::Allocator;
use oxc_parser::ParserReturn;


fn main() {
  let allocator = Allocator::new();
  let source = include_str!("example.vue");

  // get the result there
  let ret: ParserReturn = VueOxcParser::new(source, &allocator);

  let SemanticBuilderReturn { semantic, .. } = SemanticBuilder::new()
    .with_cfg(true)
    .with_scope_tree_child_ids(true)
    .with_check_syntax_error(true)
    .build(allocator.alloc(ret.program));

  semantic.set_irregular_whitespaces(ret.irregular_whitespaces);

  /* Omit the future code */
}
```

## License

This project include a fork from [vue-oxc-parser](https://github.com/zhiyuanzmj/vue-oxc-parser) originally created by zhiyuanzmj.

[MIT](./LICENSE) License - see [LICENSE](LICENSE) file for details.
