#[test]
fn namespace_secret_hex_requires_exact_length() {
    let error = crate::access::parse_namespace_secret_hex("abcd")
        .expect_err("short namespace secret should fail");
    assert!(error.to_string().contains("32 bytes"));
}
