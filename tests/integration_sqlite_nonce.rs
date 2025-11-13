mod util;

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
// tempfile helpers used via tempfile::tempdir() directly; avoid unused import

#[tokio::test]
async fn sqlite_file_backed_nonce_multi_process() -> anyhow::Result<()> {
    println!("starting sqlite_file_backed_nonce_multi_process test");
    // Create a temp file path for SQLite DB
    let tmpdir = tempfile::tempdir()?;
    let db_path = tmpdir.path().join("test_db.sqlite");
    // Ensure file exists and is writable so child processes can open it on Windows
    std::fs::File::create(&db_path)?;

    let network = "eth";
    let address = "0xmulti01";
    let count = 10usize;

    // Build paths to the harness binary (fallback to target/debug when env not set)
    let exe = std::env::var("CARGO_BIN_EXE_nonce_harness").unwrap_or_else(|_| {
        let mut path = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        path.push("defi-target");
        path.push("debug");
        let mut bin = path.join("nonce_harness");
        if cfg!(windows) {
            bin.set_extension("exe");
        }
        bin.to_string_lossy().to_string()
    });

    if !std::path::Path::new(&exe).exists() {
        panic!("nonce_harness binary not found at {}", exe);
    }

    println!("setting test env vars");
    // Use centralized test env helper; keep BRIDGE_MOCK_FORCE_SUCCESS local for this test harness
    util::set_test_env();
    std::env::set_var("BRIDGE_MOCK_FORCE_SUCCESS", "1");

    println!("spawning harness processes, db_path={}", db_path.display());
    // Spawn two processes concurrently
    let mut p1 = Command::new(&exe)
        .arg(db_path.to_string_lossy().as_ref())
        .arg(network)
        .arg(address)
        .arg(count.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut p2 = Command::new(&exe)
        .arg(db_path.to_string_lossy().as_ref())
        .arg(network)
        .arg(address)
        .arg(count.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let out1 = p1.stdout.take().expect("p1 stdout");
    let out2 = p2.stdout.take().expect("p2 stdout");

    let mut lines: Vec<u64> = Vec::new();
    for l in BufReader::new(out1).lines() {
        let s = l?;
        lines.push(s.parse::<u64>()?);
    }
    for l in BufReader::new(out2).lines() {
        let s = l?;
        lines.push(s.parse::<u64>()?);
    }

    // wait for processes to exit
    let s1 = p1.wait()?;
    let s2 = p2.wait()?;
    if !s1.success() || !s2.success() {
        // print stderr for debugging
        if let Some(mut e1) = p1.stderr.take() {
            let mut buf = String::new();
            use std::io::Read;
            let _ = e1.read_to_string(&mut buf);
            eprintln!("p1 stderr: {}", buf);
        }
        if let Some(mut e2) = p2.stderr.take() {
            let mut buf = String::new();
            use std::io::Read;
            let _ = e2.read_to_string(&mut buf);
            eprintln!("p2 stderr: {}", buf);
        }
    }
    assert!(s1.success());
    assert!(s2.success());

    // ensure we have count*2 outputs
    assert_eq!(lines.len(), count * 2);

    lines.sort_unstable();

    // ensure sequence is strictly increasing by 1
    for window in lines.windows(2) {
        assert_eq!(window[1], window[0] + 1);
    }

    Ok(())
}
