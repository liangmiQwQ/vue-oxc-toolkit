//! Cold error-constructor functions for the Vue SFC parser.
//!
//! All `OxcDiagnostic` constructions live here, keeping hot paths clean.

use oxc_diagnostics::OxcDiagnostic;

#[cold]
pub fn unexpected_eof_in_comment() -> OxcDiagnostic {
  OxcDiagnostic::error("Unexpected EOF inside comment")
}

#[cold]
pub fn unexpected_eof_in_cdata() -> OxcDiagnostic {
  OxcDiagnostic::error("Unexpected EOF inside CDATA")
}

#[cold]
pub fn unexpected_eof_in_interpolation() -> OxcDiagnostic {
  OxcDiagnostic::error("Unexpected EOF inside interpolation")
}

#[cold]
pub fn unexpected_eof_in_tag() -> OxcDiagnostic {
  OxcDiagnostic::error("Unexpected EOF in start tag")
}

#[cold]
pub fn unsupported_script_lang(lang: &str) -> OxcDiagnostic {
  OxcDiagnostic::error(format!("Unsupported script lang: {lang}"))
}

#[cold]
pub fn multiple_script_tags() -> OxcDiagnostic {
  OxcDiagnostic::error("Multiple <script> tags found")
}

#[cold]
pub fn multiple_script_setup_tags() -> OxcDiagnostic {
  OxcDiagnostic::error("Multiple <script setup> tags found")
}
