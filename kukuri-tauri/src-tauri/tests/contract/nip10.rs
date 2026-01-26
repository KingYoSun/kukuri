use kukuri_lib::contract_testing::validate_nip10_tags;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Nip10Case {
    name: String,
    #[serde(rename = "description")]
    _description: Option<String>,
    tags: Vec<Vec<String>>,
    expected: bool,
}

#[test]
fn nip10_contract_cases_align_with_rust_validation() {
    let data = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../testdata/nip10_contract_cases.json"
    ));
    let cases: Vec<Nip10Case> =
        serde_json::from_str(data).expect("nip10 contract cases json should parse");

    for case in cases {
        let result = validate_nip10_tags(case.tags.clone());
        if case.expected {
            assert!(
                result.is_ok(),
                "case '{}' expected success but got error: {:?}",
                case.name,
                result.err()
            );
        } else {
            assert!(
                result.is_err(),
                "case '{}' expected failure but succeeded",
                case.name
            );
        }
    }
}
