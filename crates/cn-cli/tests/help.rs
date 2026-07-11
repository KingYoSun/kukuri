use std::process::Command;

fn help(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_cn-cli"))
        .args(args)
        .arg("--help")
        .output()
        .expect("cn-cli must start");
    assert!(output.status.success(), "args={args:?}");
    String::from_utf8(output.stdout).expect("help output must be UTF-8")
}

#[test]
fn root_help_keeps_public_command_surface() {
    let output = help(&[]);
    for command in [
        "prepare",
        "migrate",
        "seed-policies",
        "set-auth-rollout",
        "reports",
        "admission",
        "supported-topic",
        "indexing-request",
        "relation",
    ] {
        assert!(
            output.contains(command),
            "missing `{command}` in:\n{output}"
        );
    }
    assert!(output.contains("--database-url <DATABASE_URL>"));
}

#[test]
fn command_and_nested_help_remain_parseable() {
    for args in [
        &["set-auth-rollout"][..],
        &["reports"],
        &["reports", "list"],
        &["admission"],
        &["admission", "invite"],
        &["admission", "allow"],
        &["admission", "ban"],
        &["supported-topic"],
        &["indexing-request"],
        &["relation"],
    ] {
        let output = help(args);
        assert!(output.contains("Usage:"), "args={args:?}: {output}");
    }
}
