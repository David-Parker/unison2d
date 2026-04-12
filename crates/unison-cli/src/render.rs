use anyhow::{bail, Result};
use std::collections::HashMap;

/// Substitute `{{KEY}}` tokens in `template` with corresponding values from `vars`.
/// Returns an error if a `{{KEY}}` appears in the template but is absent from `vars`.
pub fn render(template: &str, vars: &HashMap<&str, &str>) -> Result<String> {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after_open = &rest[start + 2..];
        let end = match after_open.find("}}") {
            Some(e) => e,
            None => bail!("unterminated '{{{{' in template"),
        };
        let key = after_open[..end].trim();
        match vars.get(key) {
            Some(v) => out.push_str(v),
            None => bail!("unknown template variable: {{{{{}}}}}", key),
        }
        rest = &after_open[end + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&'static str, &'static str)]) -> HashMap<&'static str, &'static str> {
        pairs.iter().copied().collect()
    }

    #[test]
    fn substitutes_single_variable() {
        let v = vars(&[("NAME", "my-game")]);
        assert_eq!(render("Hello {{NAME}}!", &v).unwrap(), "Hello my-game!");
    }

    #[test]
    fn substitutes_multiple_occurrences() {
        let v = vars(&[("X", "foo")]);
        assert_eq!(render("{{X}} and {{X}}", &v).unwrap(), "foo and foo");
    }

    #[test]
    fn substitutes_multiple_variables() {
        let v = vars(&[("A", "1"), ("B", "2")]);
        assert_eq!(render("{{A}}-{{B}}", &v).unwrap(), "1-2");
    }

    #[test]
    fn trims_whitespace_in_key() {
        let v = vars(&[("X", "y")]);
        assert_eq!(render("{{ X }}", &v).unwrap(), "y");
    }

    #[test]
    fn errors_on_unknown_variable() {
        let v = vars(&[]);
        let err = render("hello {{NAME}}", &v).unwrap_err();
        assert!(err.to_string().contains("NAME"));
    }

    #[test]
    fn errors_on_unterminated_brace() {
        let v = vars(&[]);
        assert!(render("hello {{NAME", &v).is_err());
    }

    #[test]
    fn passes_through_text_without_placeholders() {
        let v = vars(&[]);
        assert_eq!(render("plain text", &v).unwrap(), "plain text");
    }
}
