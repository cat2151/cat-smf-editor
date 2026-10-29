//! Guard the concrete dependency boundary that allowed empty handlers plus palette dispatch.
fn identifiers(source: &str) -> Vec<String> {
    let bytes = source.as_bytes();
    let mut i = 0;
    let mut out = vec![];
    while i < bytes.len() {
        if bytes[i..].starts_with(b"//") {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else if bytes[i..].starts_with(b"/*") {
            i += 2;
            let mut depth = 1;
            while i < bytes.len() && depth > 0 {
                if bytes[i..].starts_with(b"/*") {
                    depth += 1;
                    i += 2;
                } else if bytes[i..].starts_with(b"*/") {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
        } else if bytes[i] == b'"' {
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    i = (i + 2).min(bytes.len());
                } else if bytes[i] == b'"' {
                    i += 1;
                    break;
                } else {
                    i += 1;
                }
            }
        } else if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
            let start = i;
            i += 1;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            out.push(source[start..i].to_owned());
        } else {
            if !bytes[i].is_ascii_whitespace() {
                out.push((bytes[i] as char).to_string());
            }
            i += 1;
        }
    }
    out
}
fn check(source: &str) -> Result<(), String> {
    let tokens = identifiers(source);
    let forbidden = [
        "patterns",
        "pattern_args",
        "insert_event",
        "same_measure",
        "views",
        "guide_edit",
        "meta_input",
        "edit",
        "edit_music",
        "edit_tracks",
        "apply_music",
        "Operation",
        "Request",
        "Kind",
    ];
    if let Some(token) = tokens
        .iter()
        .find(|token| forbidden.contains(&token.as_str()))
    {
        return Err(format!("Palette domain dependency: {token}"));
    }
    // Converting a command ID to str was the palette's operation dispatcher.
    if tokens
        .windows(3)
        .any(|triple| triple[0] == "id" && triple[1] == "." && triple[2] == "as_str")
    {
        return Err("Palette interprets command IDs".into());
    }
    for (i, token) in tokens.iter().enumerate() {
        if token == "id"
            && tokens.get(i + 1).is_some_and(|t| t == "=")
            && tokens.get(i + 2).is_some_and(|t| t == "=")
        {
            return Err("Palette compares command IDs".into());
        }
        if token == "match" || token == "matches" {
            let expression = tokens[i + 1..]
                .iter()
                .take_while(|t| t.as_str() != "{" && t.as_str() != ")");
            if expression.clone().any(|t| t == "id") {
                return Err("Palette dispatches on command IDs".into());
            }
        }
    }
    Ok(())
}
#[test]
fn palette_production_code_has_no_operation_dispatch_or_edit_dependency() {
    for (name, source) in [
        ("palette", include_str!("../palette.rs")),
        ("history", include_str!("../palette/history.rs")),
    ] {
        check(source).unwrap_or_else(|error| panic!("{name}: {error}"));
    }
}
#[test]
fn boundary_checker_rejects_domain_examples_and_allows_search_display_and_history_ids() {
    for forbidden in [
        "use crate::view::patterns;",
        "state.pattern_args = Some(input);",
        "app.edit_music(edit);",
        "match command.id.as_str() { _ => () }",
        "use crate::edit::insert;",
        "Operation::Views",
        "if command.id == \"insert-tempo\" { run(); }",
        "matches!(command.id, \"insert-tempo\")",
    ] {
        assert!(check(forbidden).is_err(), "{forbidden}");
    }
    for allowed in [
        "let id = command.id.clone(); history.record(id);",
        "row.label(command.name);",
        "search.matches_text(\"insert-event pattern_args\");",
        "// patterns::mutate()\ncommands::execute(app,ctx,command);",
        "/* edit_music */ history.save(app);",
    ] {
        assert!(check(allowed).is_ok(), "{allowed}");
    }
}
