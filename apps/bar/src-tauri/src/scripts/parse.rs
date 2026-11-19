//! Turns a script's stdout into a bar label and a menu tree (ADR 0035). A pure function on bytes:
//! the output is untrusted, so every limit is checked here and nothing in here can panic on it.

/// Biggest stdout we accept (also enforced while reading, in `run.rs`).
pub const MAX_OUTPUT_BYTES: usize = 64 * 1024;
pub const MAX_MENU_LINES: usize = 200;
pub const MAX_LINE_CHARS: usize = 1000;
pub const MAX_LABEL_CHARS: usize = 200;
pub const MAX_HREF_CHARS: usize = 2048;
pub const MAX_SHELL_ARGS: usize = 20;
/// `--` per level, so `------` is the third.
pub const MAX_DEPTH: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellCommand {
    pub program: String,
    pub args: Vec<String>,
}

/// A clickable menu line with what its params asked for. Nothing here has run yet: `href` and
/// `shell` are checked again when the click happens.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Entry {
    pub label: String,
    pub color: Option<String>,
    pub href: Option<String>,
    pub shell: Option<ShellCommand>,
    pub refresh: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Separator,
    Item { entry: Entry, children: Vec<Node> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    pub label: String,
    pub color: Option<String>,
    pub menu: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Empty,
    TooLarge,
    /// NUL bytes or not UTF-8: binary, or the wrong encoding.
    Garbage,
}

impl ParseError {
    pub fn message(&self) -> &'static str {
        match self {
            Self::Empty => "The script printed nothing",
            Self::TooLarge => "The script printed more than 64 KB",
            Self::Garbage => "The output is not text (use UTF-8)",
        }
    }
}

pub fn parse(bytes: &[u8]) -> Result<Output, ParseError> {
    if bytes.len() > MAX_OUTPUT_BYTES {
        return Err(ParseError::TooLarge);
    }
    if bytes.contains(&0) {
        return Err(ParseError::Garbage);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| ParseError::Garbage)?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);

    let mut lines = text.lines();
    // Blank lines before the label don't count.
    let first = lines.by_ref().find(|l| !l.trim().is_empty()).ok_or(ParseError::Empty)?;
    let first = clean(first, MAX_LINE_CHARS);
    let (label_text, label_params) = split_params(&first);
    let label = truncate(label_text.trim(), MAX_LABEL_CHARS);
    let color = label_params.color;

    // Anything between the label and the first `---` is ignored for now.
    let mut flat: Vec<(usize, Option<Entry>)> = Vec::new();
    let mut in_menu = false;
    for line in lines {
        let line = clean(line, MAX_LINE_CHARS);
        let trimmed = line.trim();
        if trimmed == "---" {
            if in_menu {
                flat.push((0, None));
            }
            in_menu = true;
            continue;
        }
        if !in_menu || trimmed.is_empty() {
            continue;
        }
        if flat.len() >= MAX_MENU_LINES {
            break;
        }
        let dashes = trimmed.chars().take_while(|c| *c == '-').count();
        let depth = (dashes / 2).min(MAX_DEPTH);
        // One lone dash is just text ("- item"), only pairs make a submenu.
        let body = if depth == 0 { trimmed } else { trimmed[dashes..].trim_start() };
        let (text, params) = split_params(body);
        flat.push((depth, Some(Entry {
            label: text.trim().to_string(),
            color: params.color,
            href: params.href,
            shell: params.shell,
            refresh: params.refresh,
        })));
    }
    Ok(Output { label, color, menu: nest(&flat) })
}

/// Flat (depth, entry) lines to a tree. A line deeper than "one more than the line above" is
/// pulled up to that depth, so `--` under a top line is a child and `----` right after it too.
fn nest(flat: &[(usize, Option<Entry>)]) -> Vec<Node> {
    fn build(flat: &[(usize, Option<Entry>)], at: &mut usize, depth: usize, allowed: usize) -> Vec<Node> {
        let mut out = Vec::new();
        while *at < flat.len() {
            let (raw, entry) = &flat[*at];
            let d = (*raw).min(allowed);
            if d < depth {
                break;
            }
            *at += 1;
            let Some(entry) = entry else {
                out.push(Node::Separator);
                continue;
            };
            let children = build(flat, at, d + 1, d + 1);
            out.push(Node::Item { entry: entry.clone(), children });
        }
        out
    }
    let mut at = 0;
    let mut menu = build(flat, &mut at, 0, 0);
    tidy_separators(&mut menu);
    menu
}

/// No separator at either edge and none twice in a row.
fn tidy_separators(nodes: &mut Vec<Node>) {
    let mut last_sep = true;
    nodes.retain(|n| {
        let sep = matches!(n, Node::Separator);
        let keep = !(sep && last_sep);
        last_sep = sep;
        keep
    });
    if matches!(nodes.last(), Some(Node::Separator)) {
        nodes.pop();
    }
    for n in nodes.iter_mut() {
        if let Node::Item { children, .. } = n {
            tidy_separators(children);
        }
    }
}

/// Tabs become spaces, other control characters go, and the line is cut at `max` characters.
fn clean(line: &str, max: usize) -> String {
    line.chars()
        .map(|c| if c == '\t' { ' ' } else { c })
        .filter(|c| !c.is_control())
        .take(max)
        .collect()
}

fn truncate(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

#[derive(Default)]
struct Params {
    color: Option<String>,
    href: Option<String>,
    shell: Option<ShellCommand>,
    refresh: bool,
}

/// `text | key=value ...` into the text and its params. Params are space separated, and a ` | `
/// between them is accepted too (`a | color=red | href=...`, how xbar scripts are written). The
/// params start at the earliest ` | ` after which everything reads as `key=value` pairs; a pipe in
/// the text before that stays text. If nothing parses (bad quoting, a word with no `=`), the whole
/// line stays text.
fn split_params(line: &str) -> (&str, Params) {
    for (at, _) in line.match_indices(" | ") {
        if let Some(pairs) = parse_pairs(&line[at + 3..]) {
            return (&line[..at], build_params(pairs));
        }
    }
    (line, Params::default())
}

/// `a=1 b="two words" c='x'` into pairs. `None` on any malformed part.
fn parse_pairs(text: &str) -> Option<Vec<(String, String)>> {
    let mut pairs = Vec::new();
    let mut rest = text.trim_start();
    while !rest.is_empty() {
        // A ` | ` between two params is only a separator.
        if let Some(after) = rest.strip_prefix("| ") {
            rest = after.trim_start();
            continue;
        }
        let eq = rest.find('=')?;
        let key = &rest[..eq];
        if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric()) {
            return None;
        }
        rest = &rest[eq + 1..];
        let value;
        match rest.chars().next() {
            Some(q @ ('"' | '\'')) => {
                let close = rest[1..].find(q)?;
                value = &rest[1..1 + close];
                rest = &rest[close + 2..];
                // The closing quote must end the word.
                if !rest.is_empty() && !rest.starts_with(' ') {
                    return None;
                }
            }
            _ => {
                let end = rest.find(' ').unwrap_or(rest.len());
                value = &rest[..end];
                rest = &rest[end..];
            }
        }
        pairs.push((key.to_ascii_lowercase(), value.to_string()));
        rest = rest.trim_start();
    }
    Some(pairs)
}

fn build_params(pairs: Vec<(String, String)>) -> Params {
    let mut params = Params::default();
    let mut program = None;
    let mut numbered: Vec<(usize, String)> = Vec::new();
    for (key, value) in pairs {
        match key.as_str() {
            "color" => params.color = valid_color(&value),
            "href" => params.href = valid_href(&value),
            "refresh" => params.refresh = value.eq_ignore_ascii_case("true"),
            "shell" if !value.is_empty() => program = Some(value),
            _ => {
                if let Some(n) = key.strip_prefix("param").and_then(|n| n.parse::<usize>().ok()) {
                    if (1..=MAX_SHELL_ARGS).contains(&n) {
                        numbered.push((n, value));
                    }
                }
            }
        }
    }
    numbered.sort_by_key(|(n, _)| *n);
    // `param3` without `param2` would shift the arguments, so only a gapless run from 1 counts.
    let mut args = Vec::new();
    for (i, (n, value)) in numbered.into_iter().enumerate() {
        if n != i + 1 {
            break;
        }
        args.push(value);
    }
    params.shell = program.map(|program| ShellCommand { program, args });
    params
}

const COLOR_NAMES: &[&str] = &[
    "red", "orange", "yellow", "green", "blue", "purple", "pink", "gray", "grey", "white", "black",
    "teal", "cyan", "brown",
];

/// A name from the short list or `#rgb` / `#rrggbb`, lowercased. Anything else is dropped.
pub fn valid_color(value: &str) -> Option<String> {
    let v = value.trim().to_ascii_lowercase();
    if COLOR_NAMES.contains(&v.as_str()) {
        return Some(v);
    }
    let hex = v.strip_prefix('#')?;
    ((hex.len() == 3 || hex.len() == 6) && hex.chars().all(|c| c.is_ascii_hexdigit())).then_some(v)
}

/// Only `http`, `https` and `mailto`, no spaces or control characters. Also used again at click
/// time, so nothing that skipped the parser can open something else.
pub fn valid_href(value: &str) -> Option<String> {
    if value.is_empty()
        || value.chars().count() > MAX_HREF_CHARS
        || value.chars().any(|c| c.is_control() || c.is_whitespace())
    {
        return None;
    }
    let lower = value.to_ascii_lowercase();
    let ok = lower.strip_prefix("http://").or_else(|| lower.strip_prefix("https://")).is_some_and(|r| !r.is_empty())
        || lower.strip_prefix("mailto:").is_some_and(|r| !r.is_empty());
    ok.then(|| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(text: &str) -> Output {
        parse(text.as_bytes()).unwrap()
    }
    fn item(node: &Node) -> (&Entry, &Vec<Node>) {
        match node {
            Node::Item { entry, children } => (entry, children),
            Node::Separator => panic!("separator"),
        }
    }

    #[test]
    fn the_first_line_is_the_label() {
        let out = p("CPU 12%  RAM 63%\n---\nHello");
        assert_eq!(out.label, "CPU 12%  RAM 63%");
        assert_eq!(out.menu.len(), 1);
    }

    #[test]
    fn label_only_has_no_menu() {
        let out = p("just a label\n");
        assert_eq!(out.label, "just a label");
        assert!(out.menu.is_empty());
    }

    #[test]
    fn text_before_the_first_separator_is_ignored() {
        let out = p("label\nignored\nalso ignored\n---\nkept");
        assert_eq!(out.menu.len(), 1);
        assert_eq!(item(&out.menu[0]).0.label, "kept");
    }

    #[test]
    fn later_separators_are_separators() {
        let out = p("l\n---\na\n---\nb");
        assert!(matches!(out.menu[1], Node::Separator));
        assert_eq!(out.menu.len(), 3);
    }

    #[test]
    fn edge_and_double_separators_are_dropped() {
        let out = p("l\n---\n---\n---\na\n---\n---\n");
        assert_eq!(out.menu.len(), 1);
    }

    #[test]
    fn crlf_and_bom_and_blank_first_lines() {
        let out = parse("\u{feff}\r\n\r\nlabel\r\n---\r\nx\r\n".as_bytes()).unwrap();
        assert_eq!(out.label, "label");
        assert_eq!(item(&out.menu[0]).0.label, "x");
    }

    #[test]
    fn submenus_nest_by_dash_pairs() {
        let out = p("l\n---\nTop apps\n--chrome 8%\n--code 4%\n----deep\nAfter");
        let (top, kids) = item(&out.menu[0]);
        assert_eq!(top.label, "Top apps");
        assert_eq!(kids.len(), 2);
        let (_, deep) = item(&kids[1]);
        assert_eq!(item(&deep[0]).0.label, "deep");
        assert_eq!(item(&out.menu[1]).0.label, "After");
    }

    #[test]
    fn a_jump_in_depth_is_pulled_up_one_level() {
        let out = p("l\n---\nTop\n------way too deep");
        let (_, kids) = item(&out.menu[0]);
        assert_eq!(item(&kids[0]).0.label, "way too deep");
    }

    #[test]
    fn depth_is_capped_at_three() {
        let out = p("l\n---\na\n--b\n----c\n------d\n--------e");
        let (_, a) = item(&out.menu[0]);
        let (_, b) = item(&a[0]);
        let (_, c) = item(&b[0]);
        // Level 4 is pulled back to 3, so `e` sits next to `d`, not under it.
        assert_eq!(item(&c[0]).0.label, "d");
        assert_eq!(item(&c[1]).0.label, "e");
        assert!(item(&c[0]).1.is_empty());
    }

    #[test]
    fn a_submenu_line_before_any_parent_is_top_level() {
        let out = p("l\n---\n--orphan");
        assert_eq!(item(&out.menu[0]).0.label, "orphan");
    }

    #[test]
    fn a_lone_dash_is_text() {
        let out = p("l\n---\n- item");
        assert_eq!(item(&out.menu[0]).0.label, "- item");
    }

    #[test]
    fn params_are_read_after_the_last_pipe() {
        let out = p("l | color=red\n---\nMemory | color=orange\nA | B | href=https://example.com");
        assert_eq!(out.color.as_deref(), Some("red"));
        assert_eq!(item(&out.menu[0]).0.color.as_deref(), Some("orange"));
        let (e, _) = item(&out.menu[1]);
        assert_eq!(e.label, "A | B");
        assert_eq!(e.href.as_deref(), Some("https://example.com"));
    }

    #[test]
    fn params_can_be_separated_by_pipes_like_xbar_scripts() {
        let out = p("l
---
Open | color=red | href=https://example.com | refresh=true");
        let (e, _) = item(&out.menu[0]);
        assert_eq!(e.label, "Open");
        assert_eq!(e.color.as_deref(), Some("red"));
        assert_eq!(e.href.as_deref(), Some("https://example.com"));
        assert!(e.refresh);
    }

    #[test]
    fn a_pipe_in_the_text_stays_text_even_with_pipe_separated_params() {
        let out = p("l
---
A | B | color=red | refresh=true");
        let (e, _) = item(&out.menu[0]);
        assert_eq!(e.label, "A | B");
        assert!(e.refresh);
    }

    #[test]
    fn a_pipe_inside_quotes_is_not_a_separator() {
        let out = p("l
---
Run | shell=cmd param1=\"a | b\" | refresh=true");
        let (e, _) = item(&out.menu[0]);
        assert_eq!(e.shell.as_ref().unwrap().args, vec!["a | b"]);
        assert!(e.refresh);
    }

    #[test]
    fn shell_with_numbered_params() {
        let out = p("l\n---\nRun | shell=notepad param2=\"b c\" param1=a.txt");
        let e = item(&out.menu[0]).0;
        let shell = e.shell.as_ref().unwrap();
        assert_eq!(shell.program, "notepad");
        assert_eq!(shell.args, vec!["a.txt", "b c"]);
    }

    #[test]
    fn a_gap_in_numbered_params_stops_the_list() {
        let out = p("l\n---\nRun | shell=x param1=a param3=c");
        assert_eq!(item(&out.menu[0]).0.shell.as_ref().unwrap().args, vec!["a"]);
    }

    #[test]
    fn params_past_twenty_are_ignored() {
        let out = p("l\n---\nRun | shell=x param21=z");
        assert!(item(&out.menu[0]).0.shell.as_ref().unwrap().args.is_empty());
    }

    #[test]
    fn param_text_never_becomes_a_shell_string() {
        let out = p("l\n---\nRun | shell=cmd param1=\"/c calc & del x\"");
        let shell = item(&out.menu[0]).0.shell.as_ref().unwrap();
        assert_eq!(shell.args, vec!["/c calc & del x"]);
    }

    #[test]
    fn refresh_true_only() {
        let out = p("l\n---\na | refresh=true\nb | refresh=yes\nc | refresh=TRUE");
        assert!(item(&out.menu[0]).0.refresh);
        assert!(!item(&out.menu[1]).0.refresh);
        assert!(item(&out.menu[2]).0.refresh);
    }

    #[test]
    fn unknown_params_are_ignored() {
        let out = p("l\n---\na | image=abc dropdown=false color=blue terminal=true");
        let e = item(&out.menu[0]).0;
        assert_eq!(e.label, "a");
        assert_eq!(e.color.as_deref(), Some("blue"));
    }

    #[test]
    fn bad_quoting_keeps_the_whole_line_as_text() {
        for bad in ["a | color=\"red", "a | color=\"red\"x", "a | noequals", "a | =x", "a | b c=d"] {
            let out = p(&format!("l\n---\n{bad}"));
            let e = item(&out.menu[0]).0;
            assert_eq!(e.label, bad, "{bad}");
            assert!(e.color.is_none());
        }
    }

    #[test]
    fn colors_are_a_name_or_hex() {
        assert_eq!(valid_color("Red").as_deref(), Some("red"));
        assert_eq!(valid_color("#F80").as_deref(), Some("#f80"));
        assert_eq!(valid_color("#ff8800").as_deref(), Some("#ff8800"));
        for bad in ["", "#ff", "#gggggg", "expression(alert(1))", "red;x", "url(x)", "#ff88001"] {
            assert!(valid_color(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn hrefs_allow_only_web_and_mail() {
        assert!(valid_href("https://example.com/a?b=1").is_some());
        assert!(valid_href("HTTP://example.com").is_some());
        assert!(valid_href("mailto:a@b.c").is_some());
        for bad in [
            "", "file:///c:/windows/system32/calc.exe", "javascript:alert(1)", "ms-settings:", "calc.exe",
            "\\\\server\\share", "http://", "https://exa mple.com", "https://a.com\n", "ftp://x",
            "data:text/html,x", "steam://run/1", "mailto:",
        ] {
            assert!(valid_href(bad).is_none(), "{bad:?}");
        }
        assert!(valid_href(&format!("https://a.com/{}", "x".repeat(3000))).is_none());
    }

    #[test]
    fn an_empty_or_blank_output_is_an_error() {
        assert_eq!(parse(b""), Err(ParseError::Empty));
        assert_eq!(parse(b"  \n\t\n"), Err(ParseError::Empty));
    }

    #[test]
    fn nul_bytes_and_bad_utf8_are_garbage() {
        assert_eq!(parse(b"abc\0def"), Err(ParseError::Garbage));
        assert_eq!(parse(&[0xff, 0xfe, 0x41]), Err(ParseError::Garbage));
        // UTF-16 text (what PowerShell can print) has NULs.
        assert_eq!(parse(&[b'a', 0, b'b', 0]), Err(ParseError::Garbage));
    }

    #[test]
    fn output_over_64_kb_is_too_large() {
        assert_eq!(parse(&vec![b'a'; MAX_OUTPUT_BYTES + 1]), Err(ParseError::TooLarge));
        assert!(parse(&vec![b'a'; MAX_OUTPUT_BYTES]).is_ok());
    }

    #[test]
    fn control_characters_are_stripped() {
        let out = p("la\u{7}b\u{1b}[31mel\n---\nx\u{0085}y\tz");
        assert_eq!(out.label, "lab[31mel");
        assert_eq!(item(&out.menu[0]).0.label, "xy z");
    }

    #[test]
    fn huge_lines_are_cut() {
        let long = "x".repeat(5000);
        let out = p(&format!("{long}\n---\n{long}"));
        assert_eq!(out.label.chars().count(), MAX_LABEL_CHARS);
        assert_eq!(item(&out.menu[0]).0.label.chars().count(), MAX_LINE_CHARS);
    }

    #[test]
    fn the_menu_stops_at_200_lines() {
        let mut text = String::from("l\n---\n");
        for i in 0..1000 {
            text.push_str(&format!("line {i}\n"));
        }
        assert_eq!(p(&text).menu.len(), MAX_MENU_LINES);
    }

    #[test]
    fn multibyte_text_survives_cutting() {
        let text = "é".repeat(2000);
        let out = p(&format!("{text}\n"));
        assert_eq!(out.label.chars().count(), MAX_LABEL_CHARS);
    }

    #[test]
    fn a_pipe_param_blob_cannot_blow_up() {
        let junk = " | a=\"".repeat(500);
        let out = p(&format!("l\n---\nx{junk}"));
        assert_eq!(out.menu.len(), 1);
    }
}
