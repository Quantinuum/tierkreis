use std::{fs, process::Command};

use serde_json::{Value, json};

#[test]
fn lists_supported_tasks() {
    let output = Command::new(env!("CARGO_BIN_EXE_tkr-builtins"))
        .arg("--list")
        .output()
        .expect("run tkr-builtins");
    assert!(output.status.success());
    let names: Vec<String> = serde_json::from_slice(&output.stdout).expect("parse task list");
    assert_eq!(names, tierkreis::builtins::supported_tasks());
}

#[test]
fn executes_worker_file_protocol() {
    let directory = tempfile::tempdir().expect("create temporary directory");
    let a = directory.path().join("a.json");
    let b = directory.path().join("b.json");
    let value = directory.path().join("value.json");
    let done = directory.path().join("done");
    let error = directory.path().join("error");
    let call = directory.path().join("call.json");
    fs::write(&a, b"2").expect("write a");
    fs::write(&b, b"3").expect("write b");
    fs::write(
        &call,
        serde_json::to_vec(&json!({
            "function_name": "iadd",
            "inputs": {"a": a, "b": b},
            "outputs": {"value": value},
            "output_dir": directory.path(),
            "done_path": done,
            "error_path": error,
            "logs_path": null
        }))
        .expect("encode call"),
    )
    .expect("write call");

    let output = Command::new(env!("CARGO_BIN_EXE_tkr-builtins"))
        .arg(&call)
        .output()
        .expect("run tkr-builtins");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(done.exists());
    assert!(!error.exists());
    let result: Value =
        serde_json::from_slice(&fs::read(value).expect("read output")).expect("parse output");
    assert_eq!(result, json!(5));
}

#[test]
fn resolves_protocol_paths_from_tierkreis_directory() {
    let directory = tempfile::tempdir().expect("create temporary directory");
    let workflow = directory.path().join("workflow");
    fs::create_dir(&workflow).expect("create workflow directory");
    fs::write(workflow.join("input.json"), b"false").expect("write input");
    fs::write(
        workflow.join("call.json"),
        serde_json::to_vec(&json!({
            "function_name": "neg",
            "inputs": {"a": "workflow/input.json"},
            "outputs": {"value": "workflow/output.json"},
            "output_dir": "workflow",
            "done_path": "workflow/done",
            "error_path": "workflow/error",
            "logs_path": "workflow/logs"
        }))
        .expect("encode call"),
    )
    .expect("write call");

    let output = Command::new(env!("CARGO_BIN_EXE_tkr-builtins"))
        .env("TKR_DIR", directory.path())
        .arg("workflow/call.json")
        .output()
        .expect("run tkr-builtins");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(workflow.join("output.json")).expect("read output"),
        "true"
    );
    assert!(workflow.join("done").exists());
}

#[test]
fn records_worker_errors() {
    let directory = tempfile::tempdir().expect("create temporary directory");
    let error = directory.path().join("error");
    let call = directory.path().join("call.json");
    fs::write(
        &call,
        serde_json::to_vec(&json!({
            "function_name": "unknown",
            "inputs": {},
            "outputs": {},
            "output_dir": directory.path(),
            "done_path": directory.path().join("done"),
            "error_path": error,
            "logs_path": null
        }))
        .expect("encode call"),
    )
    .expect("write call");

    let output = Command::new(env!("CARGO_BIN_EXE_tkr-builtins"))
        .arg(&call)
        .output()
        .expect("run tkr-builtins");
    assert!(!output.status.success());
    assert!(
        fs::read_to_string(error)
            .expect("read error")
            .contains("Unknown task")
    );
}
