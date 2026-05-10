//! Unit tests for `crate::tools::vision::Vision`. Lives in a
//! sibling file to keep `vision.rs` under the 500-LOC ceiling.
use super::*;

#[test]
fn schema_lists_image_and_question_required() {
    let v = Vision::default();
    let s = v.schema();
    assert_eq!(s.name, "vision_analyze");
    let req = s.parameters["required"].as_array().unwrap();
    let names: Vec<&str> = req.iter().map(|v| v.as_str().unwrap()).collect();
    assert!(names.contains(&"image"));
    assert!(names.contains(&"question"));
}

#[test]
fn required_caps_url_form_only_includes_tool_dispatch() {
    let v = Vision::default();
    let caps = v
        .required_capabilities(&serde_json::json!({
            "image": "https://example.com/x.png",
            "question": "what?",
        }))
        .unwrap();
    assert!(caps.covers(&Capability::ToolDispatch {
        tool_name_glob: "vision_analyze".into(),
    }));
    // No FsRead in the URL case.
    assert!(!caps.covers(&Capability::FsRead {
        path_glob: "/etc/passwd".into(),
    }));
}

#[test]
fn required_caps_path_form_includes_fs_read_bounded_to_path() {
    let v = Vision::default();
    let caps = v
        .required_capabilities(&serde_json::json!({
            "image": "/tmp/argo-ws/shot.png",
            "question": "what?",
        }))
        .unwrap();
    assert!(caps.covers(&Capability::FsRead {
        path_glob: "/tmp/argo-ws/shot.png".into(),
    }));
}

#[test]
fn base64_decode_round_trip_smoke() {
    assert_eq!(base64_decode("aGVsbG8=").unwrap(), b"hello");
    assert_eq!(base64_decode("aGVs bG8").unwrap(), b"hello");
    assert!(base64_decode("aGVs!bG8=").is_err());
}

#[test]
fn mime_from_extension_known_set() {
    let p = std::path::PathBuf::from("/x/y/a.PNG");
    assert_eq!(mime_from_extension(&p).unwrap(), "image/png");
    let p = std::path::PathBuf::from("/x/y/a.jpeg");
    assert_eq!(mime_from_extension(&p).unwrap(), "image/jpeg");
    let p = std::path::PathBuf::from("/x/y/a.bmp");
    assert!(mime_from_extension(&p).is_err());
}

#[test]
fn data_url_decodes_inline_to_bytes() {
    // base64 of "hi" = "aGk="
    let img = decode_data_url("data:image/png;base64,aGk=").unwrap();
    match img {
        ResolvedImage::Bytes { bytes, mime } => {
            assert_eq!(bytes, b"hi");
            assert_eq!(mime, "image/png");
        }
        _ => panic!("expected Bytes"),
    }
}

#[test]
fn data_url_rejects_non_base64_encoding() {
    assert!(decode_data_url("data:image/png;utf8,hello").is_err());
}

#[test]
fn image_kind_classifies_inputs() {
    assert_eq!(image_kind("data:image/png;base64,xx"), ImageKind::DataUrl);
    assert_eq!(image_kind("https://x"), ImageKind::HttpsUrl);
    assert_eq!(image_kind("file:///a/b.png"), ImageKind::FileUrl);
    assert_eq!(image_kind("/a/b.png"), ImageKind::Path);
    assert_eq!(image_kind("rel/a.png"), ImageKind::Path);
}
