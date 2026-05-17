use super::*;
use flate2::read::GzDecoder;
use std::io::Read;

fn extract_entries(bytes: &[u8], compressed: bool) -> Vec<(String, Vec<u8>)> {
    let raw: Vec<u8> = if compressed {
        let mut decoder = GzDecoder::new(bytes);
        let mut buf = Vec::new();
        decoder.read_to_end(&mut buf).unwrap();
        buf
    } else {
        bytes.to_vec()
    };
    let mut archive = tar::Archive::new(raw.as_slice());
    let mut out = Vec::new();
    for entry in archive.entries().unwrap() {
        let mut e = entry.unwrap();
        let path = e.path().unwrap().to_string_lossy().to_string();
        let mut body = Vec::new();
        e.read_to_end(&mut body).unwrap();
        out.push((path, body));
    }
    out
}

#[test]
fn bundle_round_trip_lists_sorted_entries() {
    let files = vec![
        BundleFile::raw("audit/verify.txt", b"ok\n".to_vec()),
        BundleFile::json("audit/chain.jsonl", br#"{"kind":"turn_start"}"#.to_vec()),
    ];
    let bytes = build_bundle(
        files,
        "session_id = \"abc\"\n".to_string(),
        BundleOptions {
            sanitize: false,
            compressed: false,
            readme: "hello\n",
        },
    )
    .unwrap();
    let entries = extract_entries(&bytes, false);
    let names: Vec<&str> = entries.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "manifest.toml",
            "audit/chain.jsonl",
            "audit/verify.txt",
            "README.md",
        ]
    );
    let manifest = std::str::from_utf8(&entries[0].1).unwrap();
    assert!(manifest.contains("session_id = \"abc\""));
    assert!(manifest.contains("sha256 = \""));
    assert!(manifest.contains("sanitized = false"));
    assert!(manifest.contains("file_count = 2"));
}

#[test]
fn gzip_mode_emits_gzip_magic() {
    let bytes = build_bundle(
        vec![BundleFile::raw("README.txt", b"hi".to_vec())],
        String::new(),
        BundleOptions {
            sanitize: false,
            compressed: true,
            readme: "",
        },
    )
    .unwrap();
    assert_eq!(&bytes[..2], &[0x1f, 0x8b]);
    let entries = extract_entries(&bytes, true);
    let names: Vec<&str> = entries.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, vec!["manifest.toml", "README.txt"]);
}

#[test]
fn sanitize_redacts_provider_keys_and_emails() {
    let payload = br#"{"prompt":"contact ops@example.com","key":"sk-abcdefghijklmnopqrstuvwx"}"#
        .to_vec();
    let files = vec![BundleFile::json("bundle/action-1.json", payload)];
    let bytes = build_bundle(
        files,
        String::new(),
        BundleOptions {
            sanitize: true,
            compressed: false,
            readme: "",
        },
    )
    .unwrap();
    let entries = extract_entries(&bytes, false);
    let action = entries
        .iter()
        .find(|(n, _)| n == "bundle/action-1.json")
        .unwrap();
    let body = std::str::from_utf8(&action.1).unwrap();
    assert!(!body.contains("sk-abcdefghijklmnopqrstuvwx"));
    assert!(!body.contains("ops@example.com"));
    assert!(body.contains("sk-redacted-"));
    assert!(body.contains("email-redacted-"));
    // Field names survive.
    assert!(body.contains("\"prompt\""));
    assert!(body.contains("\"key\""));
    let manifest = std::str::from_utf8(&entries[0].1).unwrap();
    assert!(manifest.contains("sanitized = true"));
}

#[test]
fn checksum_changes_when_a_file_byte_flips() {
    let mk = |b: u8| {
        build_bundle(
            vec![BundleFile::raw("a.bin", vec![b])],
            String::new(),
            BundleOptions {
                sanitize: false,
                compressed: false,
                readme: "",
            },
        )
        .unwrap()
    };
    let a = mk(0x01);
    let b = mk(0x02);
    let manifest_a = std::str::from_utf8(&extract_entries(&a, false)[0].1)
        .unwrap()
        .to_string();
    let manifest_b = std::str::from_utf8(&extract_entries(&b, false)[0].1)
        .unwrap()
        .to_string();
    assert_ne!(manifest_a, manifest_b);
}
