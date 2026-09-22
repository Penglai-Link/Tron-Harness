//! Uses the same portable PTY engine as the manual terminal, without AI shell integration.
use crate::{
    application::agent::{AgentTerminal, TerminalSnapshot, TerminalStart},
    domain::agent::ToolError,
};
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::{
    collections::{HashMap, VecDeque},
    io::{Read, Write},
    sync::{Arc, Mutex},
};
const BUFFER_LIMIT: usize = 256 * 1024;
const PAGE_LIMIT: usize = 32 * 1024;
fn fail(error: impl ToString) -> ToolError {
    ToolError::ExecutionFailed(error.to_string())
}
#[derive(Default)]
struct Output {
    bytes: VecDeque<u8>,
    offset: u64,
    closed: bool,
}
struct Session {
    command: String,
    args: Vec<String>,
    owner: String,
    cwd: String,
    child: Mutex<Box<dyn Child + Send + Sync>>,
    writer: Mutex<Box<dyn Write + Send>>,
    _master: Mutex<Box<dyn MasterPty + Send>>,
    output: Arc<Mutex<Output>>,
}
impl Session {
    fn stop(&self) -> Result<(), ToolError> {
        let mut child = self.child.lock().map_err(fail)?;
        if child.try_wait().map_err(fail)?.is_none() {
            // PTY children are session leaders. Also terminate the foreground
            // job group so a shell's running server does not survive its shell.
            #[cfg(unix)]
            {
                let foreground = self._master.lock().map_err(fail)?.process_group_leader();
                let mut groups = vec![];
                if let Some(pid) = child.process_id() {
                    groups.push(pid as i32);
                }
                if let Some(group) = foreground {
                    if group > 1 && !groups.contains(&group) {
                        groups.push(group);
                    }
                }
                for group in groups {
                    let _ = std::process::Command::new("/bin/kill")
                        .args(["-KILL", "--", &format!("-{group}")])
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .status();
                }
            }
            if child.try_wait().map_err(fail)?.is_none() {
                child.kill().map_err(fail)?;
            }
            child.wait().map_err(fail)?;
        }
        Ok(())
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
#[derive(Default)]
pub struct LocalAgentTerminal {
    sessions: Mutex<HashMap<String, Arc<Session>>>,
}
impl LocalAgentTerminal {
    fn session(&self, owner: &str, id: &str) -> Result<Arc<Session>, ToolError> {
        self.sessions
            .lock()
            .map_err(fail)?
            .get(id)
            .filter(|session| session.owner == owner)
            .cloned()
            .ok_or_else(|| fail("terminal session not found in this conversation"))
    }
    fn snapshot(id: &str, session: &Session, cursor: u64) -> Result<TerminalSnapshot, ToolError> {
        let status = session
            .child
            .lock()
            .map_err(fail)?
            .try_wait()
            .map_err(fail)?;
        let output = session.output.lock().map_err(fail)?;
        let end = output.offset + output.bytes.len() as u64;
        if cursor > end {
            return Err(fail("output cursor is beyond available output"));
        }
        let start = cursor.max(output.offset);
        let mut bytes: Vec<_> = output
            .bytes
            .iter()
            .skip((start - output.offset) as usize)
            .take(PAGE_LIMIT)
            .copied()
            .collect();
        // Do not consume an incomplete UTF-8 character at a page/read boundary.
        // Invalid bytes still use lossy decoding; a closed stream cannot complete a suffix.
        if let Err(error) = std::str::from_utf8(&bytes) {
            if error.error_len().is_none() && (!output.closed || start + (bytes.len() as u64) < end)
            {
                bytes.truncate(error.valid_up_to());
            }
        }
        Ok(TerminalSnapshot {
            terminal_id: id.into(),
            command: session.command.clone(),
            args: session.args.clone(),
            cwd: session.cwd.clone(),
            running: status.is_none(),
            exit_code: status.map(|status| status.exit_code()),
            output: String::from_utf8_lossy(&bytes).into_owned(),
            next_cursor: start + bytes.len() as u64,
            truncated: cursor < output.offset,
            output_closed: output.closed,
        })
    }
}
impl AgentTerminal for LocalAgentTerminal {
    fn start(&self, request: TerminalStart) -> Result<TerminalSnapshot, ToolError> {
        let mut sessions = self.sessions.lock().map_err(fail)?;
        // Completed sessions are reclaimed only when capacity is needed.
        if sessions.len() >= 64
            || sessions
                .values()
                .filter(|session| session.owner == request.owner)
                .count()
                >= 8
        {
            sessions.retain(|_, session| {
                session
                    .child
                    .lock()
                    .ok()
                    .and_then(|mut child| child.try_wait().ok())
                    .flatten()
                    .is_none()
            });
        }
        if sessions.len() >= 64
            || sessions
                .values()
                .filter(|session| session.owner == request.owner)
                .count()
                >= 8
        {
            return Err(fail(
                "terminal capacity reached; stop existing sessions before starting more",
            ));
        }
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: 24,
                cols: 100,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(fail)?;
        let mut command = CommandBuilder::new(&request.command);
        command.args(&request.args);
        command.cwd(&request.cwd);
        command.env("TERM", "xterm-256color");
        for (key, value) in &request.env {
            command.env(key, value);
        }
        let mut child = pair.slave.spawn_command(command).map_err(fail)?;
        let reader = match pair.master.try_clone_reader() {
            Ok(reader) => reader,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(fail(error));
            }
        };
        let writer = match pair.master.take_writer() {
            Ok(writer) => writer,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(fail(error));
            }
        };
        drop(pair.slave);
        let output = Arc::new(Mutex::new(Output::default()));
        let session = Arc::new(Session {
            command: request.command,
            args: request.args,
            owner: request.owner,
            cwd: request.cwd.display().to_string(),
            child: Mutex::new(child),
            writer: Mutex::new(writer),
            _master: Mutex::new(pair.master),
            output: output.clone(),
        });
        std::thread::Builder::new()
            .name("agent-terminal-output".into())
            .spawn(move || {
                let mut reader = reader;
                let mut buffer = [0; 8192];
                while let Ok(size) = reader.read(&mut buffer) {
                    if size == 0 {
                        break;
                    }
                    let Ok(mut output) = output.lock() else {
                        return;
                    };
                    output.bytes.extend(&buffer[..size]);
                    let excess = output.bytes.len().saturating_sub(BUFFER_LIMIT);
                    output.bytes.drain(..excess);
                    output.offset += excess as u64;
                }
                if let Ok(mut output) = output.lock() {
                    output.closed = true;
                }
            })
            .map_err(fail)?;
        static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let id = format!(
            "terminal-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        );
        let snapshot = Self::snapshot(&id, &session, 0)?;
        sessions.insert(id, session);
        Ok(snapshot)
    }
    fn read(&self, owner: &str, id: &str, cursor: u64) -> Result<TerminalSnapshot, ToolError> {
        Self::snapshot(id, self.session(owner, id)?.as_ref(), cursor)
    }
    fn write(&self, owner: &str, id: &str, data: &str) -> Result<(), ToolError> {
        if data.len() > 65536 {
            return Err(fail("terminal input exceeds 64 KiB"));
        }
        let session = self.session(owner, id)?;
        if session
            .child
            .lock()
            .map_err(fail)?
            .try_wait()
            .map_err(fail)?
            .is_some()
        {
            return Err(fail("terminal process has exited"));
        }
        let mut writer = session.writer.lock().map_err(fail)?;
        writer.write_all(data.as_bytes()).map_err(fail)?;
        writer.flush().map_err(fail)
    }
    fn stop(&self, owner: &str, id: &str) -> Result<(), ToolError> {
        self.session(owner, id)?.stop()
    }

    fn list(&self, owner: &str) -> Result<Vec<TerminalSnapshot>, ToolError> {
        let sessions = self.sessions.lock().map_err(fail)?;
        sessions
            .iter()
            .filter(|(_, session)| session.owner == owner)
            .map(|(id, session)| {
                let end = {
                    let output = session.output.lock().map_err(fail)?;
                    output.offset + output.bytes.len() as u64
                };
                let mut snapshot = Self::snapshot(id, session, end)?;
                snapshot.output.clear();
                Ok(snapshot)
            })
            .collect()
    }
}
