use std::io::{self, BufRead, Write};
use std::process::ExitCode;

const RECORD_SEPARATOR: char = '\u{001e}';

fn main() -> ExitCode {
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    let mut in_text = false;
    let mut buffer = String::new();

    for line_result in stdin.lock().lines() {
        let mut line = match line_result {
            Ok(line) => line,
            Err(err) => {
                eprintln!("stdin read error: {err}");
                return ExitCode::FAILURE;
            }
        };

        if !in_text {
            if let Some(start) = find_text_open(&line) {
                in_text = true;
                line = line[start..].to_string();
            } else {
                continue;
            }
        }

        if let Some(end) = line.find("</text>") {
            buffer.push_str(&line[..end]);
            let cleaned = clean_mediawiki_text(&buffer);
            if cleaned.chars().count() > 100 {
                if writeln!(stdout, "{cleaned}{RECORD_SEPARATOR}").is_err() {
                    eprintln!("stdout write error");
                    return ExitCode::FAILURE;
                }
            }
            buffer.clear();
            in_text = false;
        } else {
            buffer.push_str(&line);
            buffer.push('\n');
        }
    }

    ExitCode::SUCCESS
}

fn find_text_open(line: &str) -> Option<usize> {
    let start = line.find("<text")?;
    let close = line[start..].find('>')?;
    Some(start + close + 1)
}

fn clean_mediawiki_text(value: &str) -> String {
    let mut out = value.to_string();
    for _ in 0..4 {
        out = remove_balanced_segments(&out, "{{", "}}");
    }
    out = strip_prefixed_wikilinks(&out, &["File:", "Сурет:"]);
    out = rewrite_wikilinks(&out);
    out = remove_self_closing_ref_tags(&out);
    out = remove_xml_sections(&out, "<ref", "</ref>");
    out = remove_xml_sections(&out, "<!--", "-->");
    out = strip_html_like_tags(&out);
    out = out.replace('\'', "");
    out = out
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !(trimmed.starts_with("==") && trimmed.ends_with("=="))
        })
        .map(|line| line.trim_start_matches(['*', '#']).trim())
        .collect::<Vec<_>>()
        .join("\n");
    normalize_whitespace(&out)
}

fn remove_balanced_segments(value: &str, open: &str, close: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut i = 0usize;
    while i < value.len() {
        if value[i..].starts_with(open) {
            if let Some(end) = find_balanced_segment_end(value, i, open, close) {
                i = end;
                continue;
            }
        }
        let ch = value[i..].chars().next().expect("char boundary");
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn find_balanced_segment_end(value: &str, start: usize, open: &str, close: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut i = start;
    while i < value.len() {
        if value[i..].starts_with(open) {
            depth += 1;
            i += open.len();
            continue;
        }
        if value[i..].starts_with(close) {
            depth = depth.saturating_sub(1);
            i += close.len();
            if depth == 0 {
                return Some(i);
            }
            continue;
        }
        let ch = value[i..].chars().next().expect("char boundary");
        i += ch.len_utf8();
    }
    None
}

fn strip_prefixed_wikilinks(value: &str, prefixes: &[&str]) -> String {
    rewrite_bracket_sections(value, |inner| {
        if prefixes.iter().any(|prefix| inner.starts_with(prefix)) {
            String::new()
        } else {
            format!("[[{inner}]]")
        }
    })
}

fn rewrite_wikilinks(value: &str) -> String {
    rewrite_bracket_sections(value, |inner| {
        if let Some((_, label)) = inner.split_once('|') {
            label.to_string()
        } else {
            inner.to_string()
        }
    })
}

fn rewrite_bracket_sections<F>(value: &str, mut f: F) -> String
where
    F: FnMut(&str) -> String,
{
    let mut out = String::with_capacity(value.len());
    let mut i = 0usize;
    while i < value.len() {
        if value[i..].starts_with("[[") {
            if let Some(end_rel) = value[i + 2..].find("]]") {
                let inner = &value[i + 2..i + 2 + end_rel];
                out.push_str(&f(inner));
                i += 2 + end_rel + 2;
                continue;
            }
        }
        let ch = value[i..].chars().next().expect("char boundary");
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn remove_self_closing_ref_tags(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut i = 0usize;
    while i < value.len() {
        if value[i..].starts_with("<ref") {
            if let Some(end_rel) = value[i..].find("/>") {
                i += end_rel + 2;
                continue;
            }
        }
        let ch = value[i..].chars().next().expect("char boundary");
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn remove_xml_sections(value: &str, open: &str, close: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut i = 0usize;
    while i < value.len() {
        if value[i..].starts_with(open) {
            if let Some(end_rel) = value[i + open.len()..].find(close) {
                i += open.len() + end_rel + close.len();
                continue;
            }
        }
        let ch = value[i..].chars().next().expect("char boundary");
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn strip_html_like_tags(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut in_tag = false;
    for ch in value.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out
}

fn normalize_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_basic_mediawiki_markup() {
        let raw = r#"
        {{temp}}
        [[File:foo.png]]
        [[Қазақстан|Қазақстан елі]]
        <ref>дерек</ref>
        ==Тақырып==
        '''мәтін'''
        "#;
        assert_eq!(clean_mediawiki_text(raw), "Қазақстан елі мәтін");
    }

    #[test]
    fn strips_nested_templates_in_multiple_passes() {
        let raw = "{{a|{{b}}}} қазақ мәтіні";
        assert_eq!(clean_mediawiki_text(raw), "қазақ мәтіні");
    }
}
