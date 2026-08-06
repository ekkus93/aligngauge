//! Minimal deterministic JSON value and renderer.

use std::collections::BTreeMap;
use std::fmt::Write as _;

/// A JSON value whose object keys are deterministically ordered.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum JsonValue {
    /// JSON `null`.
    Null,
    /// JSON Boolean.
    Bool(bool),
    /// Signed integer JSON number.
    Signed(i64),
    /// Unsigned integer JSON number.
    Unsigned(u64),
    /// JSON string.
    String(String),
    /// JSON array.
    Array(Vec<Self>),
    /// JSON object with stable key ordering.
    Object(BTreeMap<String, Self>),
}

impl JsonValue {
    /// Render compact JSON.
    #[must_use]
    pub fn to_compact_string(&self) -> String {
        let mut output = String::new();
        self.write_json(&mut output, 0, false);
        output
    }

    /// Render two-space-indented JSON with a trailing newline.
    #[must_use]
    pub fn to_pretty_string(&self) -> String {
        let mut output = String::new();
        self.write_json(&mut output, 0, true);
        output.push('\n');
        output
    }

    fn write_json(&self, output: &mut String, depth: usize, pretty: bool) {
        match self {
            Self::Null => output.push_str("null"),
            Self::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
            Self::Signed(value) => write!(output, "{value}").expect("writing to String cannot fail"),
            Self::Unsigned(value) => {
                write!(output, "{value}").expect("writing to String cannot fail");
            }
            Self::String(value) => write_escaped_string(output, value),
            Self::Array(values) => write_array(output, values, depth, pretty),
            Self::Object(values) => write_object(output, values, depth, pretty),
        }
    }
}

/// Conversion into deterministic JSON.
pub trait ToJson {
    /// Convert this value into the canonical JSON representation.
    fn to_json(&self) -> JsonValue;

    /// Render canonical pretty JSON.
    #[must_use]
    fn to_json_pretty(&self) -> String {
        self.to_json().to_pretty_string()
    }
}

impl ToJson for JsonValue {
    fn to_json(&self) -> JsonValue {
        self.clone()
    }
}

impl ToJson for String {
    fn to_json(&self) -> JsonValue {
        JsonValue::String(self.clone())
    }
}

impl ToJson for str {
    fn to_json(&self) -> JsonValue {
        JsonValue::String(self.to_owned())
    }
}

impl ToJson for bool {
    fn to_json(&self) -> JsonValue {
        JsonValue::Bool(*self)
    }
}

impl ToJson for u32 {
    fn to_json(&self) -> JsonValue {
        JsonValue::Unsigned(u64::from(*self))
    }
}

impl ToJson for u64 {
    fn to_json(&self) -> JsonValue {
        JsonValue::Unsigned(*self)
    }
}

impl<T: ToJson> ToJson for Vec<T> {
    fn to_json(&self) -> JsonValue {
        JsonValue::Array(self.iter().map(ToJson::to_json).collect())
    }
}

impl<T: ToJson> ToJson for BTreeMap<String, T> {
    fn to_json(&self) -> JsonValue {
        JsonValue::Object(
            self.iter()
                .map(|(key, value)| (key.clone(), value.to_json()))
                .collect(),
        )
    }
}

fn write_array(output: &mut String, values: &[JsonValue], depth: usize, pretty: bool) {
    output.push('[');
    if values.is_empty() {
        output.push(']');
        return;
    }

    for (index, value) in values.iter().enumerate() {
        if pretty {
            output.push('\n');
            write_indent(output, depth + 1);
        }
        value.write_json(output, depth + 1, pretty);
        if index + 1 != values.len() {
            output.push(',');
        }
    }

    if pretty {
        output.push('\n');
        write_indent(output, depth);
    }
    output.push(']');
}

fn write_object(
    output: &mut String,
    values: &BTreeMap<String, JsonValue>,
    depth: usize,
    pretty: bool,
) {
    output.push('{');
    if values.is_empty() {
        output.push('}');
        return;
    }

    for (index, (key, value)) in values.iter().enumerate() {
        if pretty {
            output.push('\n');
            write_indent(output, depth + 1);
        }
        write_escaped_string(output, key);
        output.push(':');
        if pretty {
            output.push(' ');
        }
        value.write_json(output, depth + 1, pretty);
        if index + 1 != values.len() {
            output.push(',');
        }
    }

    if pretty {
        output.push('\n');
        write_indent(output, depth);
    }
    output.push('}');
}

fn write_indent(output: &mut String, depth: usize) {
    for _ in 0..depth {
        output.push_str("  ");
    }
}

fn write_escaped_string(output: &mut String, value: &str) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0c}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            control if control <= '\u{1f}' => {
                write!(output, "\\u{:04x}", u32::from(control))
                    .expect("writing to String cannot fail");
            }
            other => output.push(other),
        }
    }
    output.push('"');
}

#[cfg(test)]
mod tests {
    use super::{JsonValue, ToJson};
    use std::collections::BTreeMap;

    #[test]
    fn object_keys_and_escaping_are_deterministic() {
        let value = JsonValue::Object(BTreeMap::from([
            (String::from("z"), "line\nvalue".to_json()),
            (String::from("a"), JsonValue::Unsigned(7)),
        ]));

        assert_eq!(
            value.to_pretty_string(),
            "{\n  \"a\": 7,\n  \"z\": \"line\\nvalue\"\n}\n"
        );
    }
}
