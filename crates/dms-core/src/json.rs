//! The bit of JSON telemetry needs (the agents' `telemetry`, the bench's, `dms-ctl`'s channel).

/// A number; NaN and the infinities as null.
pub fn num(x: f32) -> String {
    if x.is_finite() { format!("{x}") } else { "null".into() }
}

pub fn string(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

pub fn array(items: impl IntoIterator<Item = String>) -> String {
    format!("[{}]", items.into_iter().collect::<Vec<_>>().join(","))
}

/// `{"name":value,...}` of the fields, only the named ones if `only` is given.
pub fn object(fields: &[(&str, String)], only: Option<&[String]>) -> String {
    let pick = fields.iter().filter(|(k, _)| only.is_none_or(|o| o.iter().any(|n| n == k)));
    format!("{{{}}}", pick.map(|(k, v)| format!("{}:{v}", string(k))).collect::<Vec<_>>().join(","))
}
