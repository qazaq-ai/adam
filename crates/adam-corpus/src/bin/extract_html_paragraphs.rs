use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let Some(input_path) = env::args().nth(1) else {
        eprintln!("usage: cargo run -p adam-corpus --bin extract_html_paragraphs -- <input.html>");
        return ExitCode::FAILURE;
    };

    let html = match fs::read_to_string(&input_path) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("cannot read {input_path}: {err}");
            return ExitCode::FAILURE;
        }
    };

    for paragraph in extract_paragraphs(&html) {
        println!("{paragraph}");
    }
    ExitCode::SUCCESS
}

fn extract_paragraphs(html: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cursor = 0usize;
    while let Some(start_rel) = html[cursor..].find("<p") {
        let start = cursor + start_rel;
        let Some(tag_end_rel) = html[start..].find('>') else {
            break;
        };
        let body_start = start + tag_end_rel + 1;
        let Some(end_rel) = html[body_start..].find("</p>") else {
            break;
        };
        let body_end = body_start + end_rel;
        let cleaned = normalize_whitespace(&decode_entities(&strip_html_tags(
            &html[body_start..body_end],
        )));
        if !cleaned.is_empty() {
            out.push(cleaned);
        }
        cursor = body_end + "</p>".len();
    }
    out
}

fn strip_html_tags(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut inside_tag = false;
    for ch in value.chars() {
        match ch {
            '<' => inside_tag = true,
            '>' => inside_tag = false,
            _ if !inside_tag => out.push(ch),
            _ => {}
        }
    }
    out
}

fn decode_entities(value: &str) -> String {
    value
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
}

fn normalize_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_clean_paragraphs_from_html() {
        let html = r#"
            <div><p>Бірінші <b>абзац</b>.</p><p>Екінші&nbsp;абзац &amp; белгі.</p></div>
        "#;
        assert_eq!(
            extract_paragraphs(html),
            vec![
                "Бірінші абзац.".to_string(),
                "Екінші абзац & белгі.".to_string()
            ]
        );
    }
}
