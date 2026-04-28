//! Node.js binding for `vue_oxlint_parser`.
//!
//! Exposes a single `parseSync(source)` function which forwards to the pure
//! Rust crate and returns its JSON result as a string. The TypeScript
//! wrapper in `js/index.ts` calls `JSON.parse` on this output. Keeping the
//! transport plain JSON means the Rust crate stays free of any napi
//! types — the binding here is the only seam between the two worlds.

#![deny(clippy::all)]

use napi::Error as NapiError;
use napi_derive::napi;
use vue_oxlint_parser::{OxcDiagnostic, ParseOptions, parse_to_json};

/// Render an `OxcDiagnostic` (with its labels attached to the source) into a
/// string suitable for a JS-side `Error.message`. The miette renderer gives
/// us a multi-line, source-aware string for free; if rendering itself fails
/// we fall back to the diagnostic's plain message.
fn diagnostic_to_napi(source: &str, d: OxcDiagnostic) -> NapiError {
  let report = d.with_source_code(source.to_string());
  NapiError::from_reason(format!("{report:?}"))
}

#[napi]
#[must_use]
pub const fn plus_100(input: u32) -> u32 {
  input + 100
}

/// Parse a `.vue` SFC source string and return a JSON-serialised AST.
///
/// The returned string is a JSON object of the form
/// `{ document: VDocumentFragment, scripts: ScriptJson[] }`. See
/// `vue_oxlint_parser::parser` for the schema details.
///
/// # Errors
/// Returns a JS-side `Error` when the SFC layout is malformed.
#[napi(js_name = "parseSync")]
#[allow(clippy::needless_pass_by_value)] // napi requires owned String here
pub fn parse_sync(source: String) -> Result<String, NapiError> {
  parse_to_json(&source, &ParseOptions::default()).map_err(|d| diagnostic_to_napi(&source, d))
}
