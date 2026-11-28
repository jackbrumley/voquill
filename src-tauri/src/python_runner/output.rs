//! The Python runner's error output (Python logging, uvicorn and tracebacks
//! all write there), read for as long as the runner lives.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::ChildStderr;

/// How many of the runner's latest output lines an early exit reports.
const RUNNER_OUTPUT_TAIL_LINES: usize = 40;

/// The runner's error output (Python logging, uvicorn and tracebacks all
/// write there), read for as long as the runner lives. Left unread, the pipe
/// fills after enough requests and the runner blocks on its next log line.
pub struct RunnerOutput {
    /// The latest lines, so an early exit can say why.
    recent_lines: Arc<std::sync::Mutex<VecDeque<String>>>,
    /// Ends when the runner closes its output, i.e. when it exits.
    reader: tokio::task::JoinHandle<()>,
}

impl RunnerOutput {
    /// Everything the runner printed before it exited, once it has all been read.
    pub async fn final_lines(self) -> String {
        let _ = tokio::time::timeout(Duration::from_secs(1), self.reader).await;
        let lines = self.recent_lines.lock().unwrap();
        lines.iter().cloned().collect::<Vec<_>>().join("\n")
    }
}

/// Adds `line`, dropping the oldest once `RUNNER_OUTPUT_TAIL_LINES` are kept.
fn keep_latest(recent: &mut VecDeque<String>, line: String) {
    if recent.len() == RUNNER_OUTPUT_TAIL_LINES {
        recent.pop_front();
    }
    recent.push_back(line);
}

/// Copies every line the runner writes to the session log and keeps the
/// latest ones for `RunnerOutput::final_lines`.
pub fn drain_runner_output(stderr: ChildStderr) -> RunnerOutput {
    let recent_lines = Arc::new(std::sync::Mutex::new(VecDeque::new()));
    let reader_lines = Arc::clone(&recent_lines);
    let reader = tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            crate::log_info!("[Python runner] {}", line);
            keep_latest(&mut reader_lines.lock().unwrap(), line);
        }
    });
    RunnerOutput {
        recent_lines,
        reader,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio;

    /// `git` with an unknown option writes to stderr and exits, on every
    /// platform CI runs.
    async fn failing_process_output() -> RunnerOutput {
        let mut child = tokio::process::Command::new("git")
            .arg("--not-a-real-option")
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("git is installed");
        let output = drain_runner_output(child.stderr.take().unwrap());
        child.wait().await.unwrap();
        output
    }

    #[tokio::test]
    async fn final_lines_report_what_the_process_printed_before_exiting() {
        let lines = failing_process_output().await.final_lines().await;
        assert!(
            lines.contains("--not-a-real-option"),
            "unexpected output: {lines}"
        );
    }

    #[test]
    fn only_the_latest_lines_are_kept() {
        let mut recent = VecDeque::new();
        for number in 0..RUNNER_OUTPUT_TAIL_LINES + 5 {
            keep_latest(&mut recent, format!("line {number}"));
        }
        assert_eq!(recent.len(), RUNNER_OUTPUT_TAIL_LINES);
        assert_eq!(recent.front().unwrap(), "line 5");
        assert_eq!(
            recent.back().unwrap(),
            &format!("line {}", RUNNER_OUTPUT_TAIL_LINES + 4)
        );
    }
}
