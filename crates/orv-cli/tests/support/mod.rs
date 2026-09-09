use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const DAP_RESPONSE_TIMEOUT: Duration = Duration::from_secs(10);
static TEST_DIR_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Time alone can collide between parallel tests; the process-wide sequence cannot.
pub fn temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let sequence = TEST_DIR_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "orv-cli-{name}-{}-{nanos}-{sequence}",
        std::process::id()
    ))
}

pub const fn orv_bin() -> &'static str {
    env!("CARGO_BIN_EXE_orv")
}

pub fn orv_output(args: &[&str]) -> std::process::Output {
    Command::new(orv_bin())
        .args(args)
        .output()
        .expect("run orv")
}

pub fn assert_success(output: &std::process::Output, context: &str) {
    assert!(
        output.status.success(),
        "{context} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

pub fn run_orv(args: &[&str]) {
    assert_success(&orv_output(args), &format!("orv {args:?}"));
}

pub fn run_orv_json(args: &[&str]) -> serde_json::Value {
    let output = orv_output(args);
    assert_success(&output, &format!("orv {args:?}"));
    serde_json::from_slice(&output.stdout).expect("orv JSON stdout")
}

pub fn run_orv_expect_failure(args: &[&str]) -> String {
    let output = orv_output(args);
    assert!(
        !output.status.success(),
        "orv {args:?} unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stderr).into_owned()
}

pub fn read_text(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

pub fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_str(&read_text(path))
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()))
}

pub fn assert_keys(value: &serde_json::Value, expected: &[&str], context: &str) {
    let object = value
        .as_object()
        .unwrap_or_else(|| panic!("{context} must be an object"));
    let actual = object
        .keys()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    let expected = expected
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(actual, expected, "{context} keys drifted");
}

/// A unique test directory that is removed even when the test unwinds.
pub struct TestDir {
    path: PathBuf,
}

impl TestDir {
    pub fn new(name: &str) -> Self {
        let path = temp_dir(name);
        std::fs::create_dir_all(&path).expect("create test directory");
        Self { path }
    }
}

impl Deref for TestDir {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl AsRef<Path> for TestDir {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Keeps an ephemeral localhost port reserved until the runtime is ready to bind it.
pub struct PortReservation {
    listener: TcpListener,
}

impl PortReservation {
    pub fn localhost() -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("reserve localhost port");
        Self { listener }
    }

    pub fn port(&self) -> u16 {
        self.listener
            .local_addr()
            .expect("reserved port address")
            .port()
    }
}

struct ChildGuard(Option<Child>);

impl ChildGuard {
    fn disarm(mut self) -> Child {
        self.0.take().expect("guarded child")
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(child) = self.0.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// A DAP child process whose stdout is decoded off-thread so requests can time out.
pub struct DapServer {
    child: Child,
    stdin: ChildStdin,
    frames: Receiver<Result<serde_json::Value, String>>,
    stderr: Arc<Mutex<String>>,
    stdout_reader: Option<JoinHandle<()>>,
    stderr_reader: Option<JoinHandle<()>>,
}

impl DapServer {
    pub fn start() -> Self {
        let mut child = ChildGuard(Some(
            Command::new(env!("CARGO_BIN_EXE_orv"))
                .args(["dap", "serve", "--stdio"])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("spawn DAP server"),
        ));
        let stdin = child
            .0
            .as_mut()
            .expect("guarded child")
            .stdin
            .take()
            .expect("DAP stdin");
        let stdout = child
            .0
            .as_mut()
            .expect("guarded child")
            .stdout
            .take()
            .expect("DAP stdout");
        let stderr_pipe = child
            .0
            .as_mut()
            .expect("guarded child")
            .stderr
            .take()
            .expect("DAP stderr");

        let (frame_tx, frames) = mpsc::channel();
        let stdout_reader = thread::spawn(move || {
            let mut stdout = BufReader::new(stdout);
            loop {
                match read_dap_frame(&mut stdout) {
                    Ok(Some(frame)) => {
                        if frame_tx.send(Ok(frame)).is_err() {
                            return;
                        }
                    }
                    Ok(None) => {
                        let _ = frame_tx.send(Err("DAP stdout closed".to_string()));
                        return;
                    }
                    Err(error) => {
                        let _ = frame_tx.send(Err(error));
                        return;
                    }
                }
            }
        });

        let stderr = Arc::new(Mutex::new(String::new()));
        let stderr_output = Arc::clone(&stderr);
        let stderr_reader = thread::spawn(move || {
            let mut reader = BufReader::new(stderr_pipe);
            let mut line = String::new();
            while reader.read_line(&mut line).unwrap_or(0) != 0 {
                stderr_output
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .push_str(&line);
                line.clear();
            }
        });

        Self {
            child: child.disarm(),
            stdin,
            frames,
            stderr,
            stdout_reader: Some(stdout_reader),
            stderr_reader: Some(stderr_reader),
        }
    }

    pub fn request(&mut self, request: &serde_json::Value) -> serde_json::Value {
        let request_seq = request["seq"].as_u64().expect("request seq");
        let body = serde_json::to_vec(request).expect("serialize request");
        write!(self.stdin, "Content-Length: {}\r\n\r\n", body.len()).expect("write DAP header");
        self.stdin.write_all(&body).expect("write DAP body");
        self.stdin.flush().expect("flush DAP request");

        let deadline = Instant::now() + DAP_RESPONSE_TIMEOUT;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                self.response_failure(request_seq, "timed out waiting for response");
            }
            match self.frames.recv_timeout(remaining) {
                Ok(Ok(frame))
                    if frame["type"] == "response" && frame["request_seq"] == request_seq =>
                {
                    return frame;
                }
                Ok(Ok(_)) => {}
                Ok(Err(error)) => self.response_failure(request_seq, &error),
                Err(RecvTimeoutError::Timeout) => {
                    self.response_failure(request_seq, "timed out waiting for response");
                }
                Err(RecvTimeoutError::Disconnected) => {
                    self.response_failure(request_seq, "DAP response reader disconnected");
                }
            }
        }
    }

    fn response_failure(&mut self, request_seq: u64, reason: &str) -> ! {
        let status = self.child.try_wait().ok().flatten();
        let stderr = self
            .stderr
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        panic!(
            "DAP request {request_seq} failed: {reason}; child_status={status:?}; stderr={stderr:?}"
        );
    }
}

impl Drop for DapServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.stdout_reader.take() {
            let _ = reader.join();
        }
        if let Some(reader) = self.stderr_reader.take() {
            let _ = reader.join();
        }
    }
}

fn read_dap_frame(reader: &mut impl BufRead) -> Result<Option<serde_json::Value>, String> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        let read = reader
            .read_line(&mut line)
            .map_err(|error| format!("read DAP header: {error}"))?;
        if read == 0 {
            return Ok(None);
        }
        let header = line.trim_end_matches(['\n', '\r']);
        if header.is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            if name.eq_ignore_ascii_case("Content-Length") {
                content_length = Some(
                    value
                        .trim()
                        .parse::<usize>()
                        .map_err(|error| format!("invalid DAP content length: {error}"))?,
                );
            }
        }
    }

    let length = content_length.ok_or_else(|| "missing DAP content length".to_string())?;
    let mut body = vec![0_u8; length];
    reader
        .read_exact(&mut body)
        .map_err(|error| format!("read DAP body: {error}"))?;
    serde_json::from_slice(&body)
        .map(Some)
        .map_err(|error| format!("parse DAP frame: {error}"))
}
