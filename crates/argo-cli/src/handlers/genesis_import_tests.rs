use super::*;

fn args(source: PathBuf, name: &str, output: PathBuf, format: ImportFormat) -> GenesisImportArgs {
    GenesisImportArgs {
        source,
        name: name.to_string(),
        format,
        output,
        json: true,
    }
}

#[test]
fn import_langchain_python_extracts_template_string() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("agent.py");
    std::fs::write(
        &src,
        "from langchain import PromptTemplate\n\
         tmpl = PromptTemplate(\n    template=\"\"\"You are SAGE, a careful research assistant.\n\
         Question: {q}\nAnswer:\"\"\",\n    input_variables=[\"q\"],\n)\n",
    )
    .unwrap();
    let out = dir.path().join("verticals");

    handle_import(args(src, "sage-research", out.clone(), ImportFormat::Auto)).unwrap();

    let prompt = std::fs::read_to_string(out.join("sage-research/system-prompt.md")).unwrap();
    assert!(prompt.contains("SAGE"), "prompt: {prompt}");
    assert!(prompt.contains("Question: {q}"), "prompt: {prompt}");
}

#[test]
fn import_markdown_extracts_first_fenced_code_block() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("prompt.md");
    std::fs::write(
        &src,
        "# Customer support agent\n\nThis is intro prose.\n\n```\nYou are CARA, a calm support agent.\nAlways acknowledge the issue first.\n```\n\nMore notes here.\n",
    )
    .unwrap();
    let out = dir.path().join("verticals");

    handle_import(args(src, "cara-support", out.clone(), ImportFormat::Auto)).unwrap();

    let prompt = std::fs::read_to_string(out.join("cara-support/system-prompt.md")).unwrap();
    assert!(prompt.starts_with("You are CARA"), "prompt: {prompt}");
    assert!(!prompt.contains("intro prose"), "prompt: {prompt}");
    assert!(!prompt.contains("More notes"), "prompt: {prompt}");
}

#[test]
fn import_markdown_without_fence_falls_back_to_h1_plus_body() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("prompt.md");
    std::fs::write(
        &src,
        "# Sales SDR\n\nYou are SAL, an outbound sales rep.\nFollow the cold-email playbook.\n",
    )
    .unwrap();
    let out = dir.path().join("verticals");

    handle_import(args(src, "sal-sdr", out.clone(), ImportFormat::Markdown)).unwrap();

    let prompt = std::fs::read_to_string(out.join("sal-sdr/system-prompt.md")).unwrap();
    assert!(prompt.contains("# Sales SDR"));
    assert!(prompt.contains("outbound sales rep"));
}

#[test]
fn import_raw_text_uses_full_body_verbatim() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("prompt.txt");
    let body = "You are RAY, the receptionist.\nGreet every visitor.\n";
    std::fs::write(&src, body).unwrap();
    let out = dir.path().join("verticals");

    handle_import(args(src, "ray-recep", out.clone(), ImportFormat::Auto)).unwrap();

    let prompt = std::fs::read_to_string(out.join("ray-recep/system-prompt.md")).unwrap();
    assert!(prompt.contains("RAY"));
    assert!(prompt.contains("Greet every visitor"));
}

#[test]
fn auto_detect_picks_langchain_for_py_files() {
    assert_eq!(
        resolve_format(ImportFormat::Auto, Path::new("foo.py")),
        ImportFormat::Langchain,
    );
    assert_eq!(
        resolve_format(ImportFormat::Auto, Path::new("foo.md")),
        ImportFormat::Markdown,
    );
    assert_eq!(
        resolve_format(ImportFormat::Auto, Path::new("foo.txt")),
        ImportFormat::Raw,
    );
    assert_eq!(
        resolve_format(ImportFormat::Auto, Path::new("foo")),
        ImportFormat::Raw,
    );
}

#[test]
fn import_missing_source_returns_provider_error() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("verticals");

    let err = handle_import(args(
        dir.path().join("does-not-exist.txt"),
        "ghost",
        out,
        ImportFormat::Auto,
    ))
    .expect_err("missing source");
    let msg = err.to_string();
    assert!(msg.contains("source not found"), "got: {msg}");
}

#[test]
fn import_existing_vertical_name_refuses_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("prompt.txt");
    std::fs::write(&src, "first prompt body").unwrap();
    let out = dir.path().join("verticals");

    handle_import(args(
        src.clone(),
        "shared",
        out.clone(),
        ImportFormat::Auto,
    ))
    .unwrap();

    let err = handle_import(args(src, "shared", out, ImportFormat::Auto))
        .expect_err("second import with same name");
    let msg = err.to_string();
    assert!(msg.contains("already exists"), "got: {msg}");
}

#[test]
fn invalid_name_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("p.txt");
    std::fs::write(&src, "body").unwrap();
    let out = dir.path().join("verticals");

    let err = handle_import(args(src, "Bad_Name", out, ImportFormat::Auto))
        .expect_err("invalid name");
    assert!(err.to_string().contains("must be lowercase ASCII"));
}

#[test]
fn manifest_parses_as_valid_vertical_toml() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("p.txt");
    std::fs::write(&src, "be helpful").unwrap();
    let out = dir.path().join("verticals");

    handle_import(args(src, "smoke-vert", out.clone(), ImportFormat::Auto)).unwrap();

    let body = std::fs::read_to_string(out.join("smoke-vert/vertical.toml")).unwrap();
    let manifest: argo_vertical::VerticalManifest =
        toml::from_str(&body).expect("vertical.toml parses");
    assert_eq!(manifest.vertical.id, "smoke-vert");
    assert_eq!(manifest.schema_version, "1");
}
