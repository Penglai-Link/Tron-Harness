use std::{
    collections::{HashMap, VecDeque},
    io::{Read, Write},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use crate::domain::agent::{AgentRunContext, Tool, ToolError, ToolExecutionResult, ToolRequest};
use serde::{Deserialize, Serialize};

use super::{
    EnvVars, default_cwd, json_observation, parse_tool_input, resolve_command_invocation,
    resolve_optional_directory,
};

const FOREGROUND_LIMIT: Duration = Duration::from_secs(10);
const POLL_INTERVAL: Duration = Duration::from_millis(100);
const BUFFER_LIMIT: usize = 128 * 1024;
const PAGE_LIMIT: usize = 32 * 1024;
const MAX_TASKS: usize = 64;
const MAX_TASKS_PER_OWNER: usize = 8;

#[derive(Default)]
struct StreamBuffer {
    bytes: VecDeque<u8>,
    offset: u64,
}

#[derive(Default)]
struct CommandOutput {
    stdout: StreamBuffer,
    stderr: StreamBuffer,
    closed_readers: usize,
}

struct CommandTask {
    task_id: String,
    owner: String,
    command: String,
    args: Vec<String>,
    cwd: String,
    child: Mutex<Child>,
    output: Arc<Mutex<CommandOutput>>,
}

impl CommandTask {
    fn running(&self) -> Result<bool, ToolError> {
        self.child
            .lock()
            .map_err(fail)?
            .try_wait()
            .map(|status| status.is_none())
            .map_err(fail)
    }

    fn complete(&self) -> Result<bool, ToolError> {
        Ok(!self.running()? && self.output.lock().map_err(fail)?.closed_readers == 2)
    }

    fn stop(&self) -> Result<bool, ToolError> {
        let mut child = self.child.lock().map_err(fail)?;
        if child.try_wait().map_err(fail)?.is_none() {
            child.kill().map_err(fail)?;
            child.wait().map_err(fail)?;
            return Ok(true);
        }
        Ok(false)
    }
}

impl Drop for CommandTask {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[derive(Default)]
struct CommandTaskManager {
    tasks: Mutex<HashMap<String, Arc<CommandTask>>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CommandTaskSnapshot {
    task_id: String,
    command: String,
    args: Vec<String>,
    cwd: String,
    running: bool,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
    next_stdout_cursor: u64,
    next_stderr_cursor: u64,
    truncated: bool,
    output_closed: bool,
}

impl CommandTaskManager {
    fn start(
        &self,
        owner: String,
        command: String,
        args: Vec<String>,
        cwd: std::path::PathBuf,
        stdin: Option<String>,
        env: Option<EnvVars>,
    ) -> Result<CommandTaskSnapshot, ToolError> {
        let mut tasks = self.tasks.lock().map_err(fail)?;
        if tasks.len() >= MAX_TASKS
            || tasks.values().filter(|task| task.owner == owner).count() >= MAX_TASKS_PER_OWNER
        {
            tasks.retain(|_, task| !task.complete().unwrap_or(false));
        }
        if tasks.len() >= MAX_TASKS
            || tasks.values().filter(|task| task.owner == owner).count() >= MAX_TASKS_PER_OWNER
        {
            return Err(fail(
                "background command capacity reached; wait for or stop existing tasks",
            ));
        }

        let mut process = Command::new(&command);
        process
            .args(&args)
            .current_dir(&cwd)
            .stdin(if stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(env) = &env {
            process.envs(env.iter());
        }
        let mut child = process
            .spawn()
            .map_err(|error| fail(format!("command failed to start: {error}")))?;
        if let Some(stdin) = stdin {
            if let Some(mut writer) = child.stdin.take() {
                writer
                    .write_all(stdin.as_bytes())
                    .map_err(|error| fail(format!("command stdin write failed: {error}")))?;
            }
        }
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| fail("command stdout is not available"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| fail("command stderr is not available"))?;

        let output = Arc::new(Mutex::new(CommandOutput::default()));
        spawn_reader(stdout, output.clone(), true);
        spawn_reader(stderr, output.clone(), false);
        static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let task_id = format!(
            "command-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        );
        let task = Arc::new(CommandTask {
            task_id: task_id.clone(),
            owner,
            command,
            args,
            cwd: cwd.display().to_string(),
            child: Mutex::new(child),
            output,
        });
        let snapshot = Self::snapshot(&task, 0, 0)?;
        tasks.insert(task_id, task);
        Ok(snapshot)
    }

    fn read(
        &self,
        owner: &str,
        task_id: &str,
        stdout_cursor: u64,
        stderr_cursor: u64,
    ) -> Result<CommandTaskSnapshot, ToolError> {
        let task = self
            .tasks
            .lock()
            .map_err(fail)?
            .get(task_id)
            .filter(|task| task.owner == owner)
            .cloned()
            .ok_or_else(|| fail("background command task not found in this conversation"))?;
        Self::snapshot(&task, stdout_cursor, stderr_cursor)
    }

    fn stop(&self, owner: &str, task_id: &str) -> Result<bool, ToolError> {
        let task = self
            .tasks
            .lock()
            .map_err(fail)?
            .get(task_id)
            .filter(|task| task.owner == owner)
            .cloned()
            .ok_or_else(|| fail("background command task not found in this conversation"))?;
        task.stop()
    }

    fn snapshot(
        task: &CommandTask,
        stdout_cursor: u64,
        stderr_cursor: u64,
    ) -> Result<CommandTaskSnapshot, ToolError> {
        let status = task.child.lock().map_err(fail)?.try_wait().map_err(fail)?;
        let output = task.output.lock().map_err(fail)?;
        let (stdout, next_stdout_cursor, stdout_truncated) =
            read_stream(&output.stdout, stdout_cursor)?;
        let (stderr, next_stderr_cursor, stderr_truncated) =
            read_stream(&output.stderr, stderr_cursor)?;
        Ok(CommandTaskSnapshot {
            task_id: task.task_id.clone(),
            command: task.command.clone(),
            args: task.args.clone(),
            cwd: task.cwd.clone(),
            running: status.is_none(),
            exit_code: status.and_then(|status| status.code()),
            stdout,
            stderr,
            next_stdout_cursor,
            next_stderr_cursor,
            truncated: stdout_truncated || stderr_truncated,
            output_closed: output.closed_readers == 2,
        })
    }
}

fn spawn_reader<R>(mut reader: R, output: Arc<Mutex<CommandOutput>>, is_stdout: bool)
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut buffer = [0; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(size) => {
                    let Ok(mut output) = output.lock() else {
                        return;
                    };
                    let stream = if is_stdout {
                        &mut output.stdout
                    } else {
                        &mut output.stderr
                    };
                    stream.bytes.extend(&buffer[..size]);
                    let excess = stream.bytes.len().saturating_sub(BUFFER_LIMIT);
                    stream.bytes.drain(..excess);
                    stream.offset += excess as u64;
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
        if let Ok(mut output) = output.lock() {
            output.closed_readers += 1;
        }
    });
}

fn read_stream(stream: &StreamBuffer, cursor: u64) -> Result<(String, u64, bool), ToolError> {
    let end = stream.offset + stream.bytes.len() as u64;
    if cursor > end {
        return Err(fail("output cursor is beyond available output"));
    }
    let start = cursor.max(stream.offset);
    let mut bytes: Vec<_> = stream
        .bytes
        .iter()
        .skip((start - stream.offset) as usize)
        .take(PAGE_LIMIT)
        .copied()
        .collect();
    if let Err(error) = std::str::from_utf8(&bytes) {
        if error.error_len().is_none() && start + (bytes.len() as u64) < end {
            bytes.truncate(error.valid_up_to());
        }
    }
    let next_cursor = start + bytes.len() as u64;
    Ok((
        String::from_utf8_lossy(&bytes).into_owned(),
        next_cursor,
        cursor < stream.offset,
    ))
}

fn fail(error: impl ToString) -> ToolError {
    ToolError::ExecutionFailed(error.to_string())
}

fn owner(context: &AgentRunContext) -> String {
    serde_json::to_string(&(
        &context.workspace_code,
        context.user_code.as_deref(),
        context
            .conversation_code
            .as_deref()
            .unwrap_or(&context.run_code),
    ))
    .expect("command task owner fields serialize")
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BashExecInput {
    command: String,
    #[serde(default)]
    args: Option<Vec<String>>,
    cwd: Option<String>,
    stdin: Option<String>,
    env: Option<EnvVars>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BashExecOutput {
    command: String,
    args: Vec<String>,
    cwd: String,
    success: Option<bool>,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
    task_id: Option<String>,
    running: bool,
    next_stdout_cursor: u64,
    next_stderr_cursor: u64,
    truncated: bool,
}

struct BashExecTool(Arc<CommandTaskManager>);

impl Tool for BashExecTool {
    fn name(&self) -> &str {
        "bash_exec"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let mut progress = |_line: String| {};
        self.exec_with_progress(request, context, &mut progress)
    }

    fn exec_with_progress(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
        progress: &mut dyn FnMut(String),
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: BashExecInput = parse_tool_input(request.input)?;
        let command = input.command.trim();
        if command.is_empty() {
            return Err(fail("command is required"));
        }
        let cwd = resolve_optional_directory(context, input.cwd.as_deref())?
            .unwrap_or_else(default_cwd)
            .canonicalize()
            .map_err(fail)?;
        let (program, arguments) =
            resolve_command_invocation(command, input.args.as_deref(), &cwd)?;
        let task = self.0.start(
            owner(context),
            program,
            arguments,
            cwd.clone(),
            input.stdin,
            input.env,
        )?;
        let deadline = Instant::now() + FOREGROUND_LIMIT;
        let task_id = task.task_id.clone();
        let mut snapshot = task;
        let mut stdout = snapshot.stdout.clone();
        let mut stderr = snapshot.stderr.clone();
        let mut stdout_cursor = snapshot.next_stdout_cursor;
        let mut stderr_cursor = snapshot.next_stderr_cursor;
        let mut truncated = snapshot.truncated;
        publish_output(&snapshot.stdout, &snapshot.stderr, progress);

        while (snapshot.running || !snapshot.output_closed) && Instant::now() < deadline {
            thread::sleep(POLL_INTERVAL);
            snapshot = self
                .0
                .read(&owner(context), &task_id, stdout_cursor, stderr_cursor)?;
            stdout.push_str(&snapshot.stdout);
            stderr.push_str(&snapshot.stderr);
            publish_output(&snapshot.stdout, &snapshot.stderr, progress);
            stdout_cursor = snapshot.next_stdout_cursor;
            stderr_cursor = snapshot.next_stderr_cursor;
            truncated |= snapshot.truncated;
        }

        json_observation(
            self.name(),
            &BashExecOutput {
                command: command.into(),
                args: input.args.unwrap_or_default(),
                cwd: cwd.display().to_string(),
                success: if snapshot.running || !snapshot.output_closed {
                    None
                } else {
                    Some(snapshot.exit_code == Some(0))
                },
                exit_code: snapshot.exit_code,
                stdout,
                stderr,
                task_id: (snapshot.running || !snapshot.output_closed).then_some(task_id),
                running: snapshot.running,
                next_stdout_cursor: stdout_cursor,
                next_stderr_cursor: stderr_cursor,
                truncated,
            },
        )
    }
}

fn publish_output(stdout: &str, stderr: &str, progress: &mut dyn FnMut(String)) {
    for line in stdout.lines().filter(|line| !line.trim().is_empty()) {
        progress(line.to_string());
    }
    for line in stderr.lines().filter(|line| !line.trim().is_empty()) {
        progress(format!("[stderr] {line}"));
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BashTaskReadInput {
    task_id: String,
    #[serde(default)]
    stdout_cursor: u64,
    #[serde(default)]
    stderr_cursor: u64,
}

struct BashTaskReadTool(Arc<CommandTaskManager>);

impl Tool for BashTaskReadTool {
    fn name(&self) -> &str {
        "bash_task_read"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: BashTaskReadInput = parse_tool_input(request.input)?;
        if input.task_id.trim().is_empty() {
            return Err(fail("taskId is required"));
        }
        let snapshot = self.0.read(
            &owner(context),
            &input.task_id,
            input.stdout_cursor,
            input.stderr_cursor,
        )?;
        json_observation(self.name(), &snapshot)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BashTaskStopInput {
    task_id: String,
}

struct BashTaskStopTool(Arc<CommandTaskManager>);

impl Tool for BashTaskStopTool {
    fn name(&self) -> &str {
        "bash_task_stop"
    }

    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let input: BashTaskStopInput = parse_tool_input(request.input)?;
        if input.task_id.trim().is_empty() {
            return Err(fail("taskId is required"));
        }
        let stopped = self.0.stop(&owner(context), &input.task_id)?;
        json_observation(
            self.name(),
            &serde_json::json!({"taskId": input.task_id, "stopped": stopped}),
        )
    }
}

pub(super) fn tools() -> Vec<Arc<dyn Tool>> {
    let manager = Arc::new(CommandTaskManager::default());
    vec![
        Arc::new(BashExecTool(manager.clone())),
        Arc::new(BashTaskReadTool(manager.clone())),
        Arc::new(BashTaskStopTool(manager)),
    ]
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::domain::agent::{
        AgentExecutionScope, AgentKind, AgentMessage, AgentModelConfig, AgentModelProtocol,
        AgentRunContext, AgentRunLimits, ToolRequest,
    };

    use super::*;

    fn test_context() -> AgentRunContext {
        AgentRunContext {
            auto_model_route: None,
            conversation_code: None,
            run_code: "RUN-BASH-TEST".into(),
            workspace_code: "TRON_CORE".into(),
            user_code: None,
            locale: "en-US".into(),
            request_id: None,
            stream_key: None,
            agent_kind: AgentKind::React,
            system_agent_session_id: None,
            model: AgentModelConfig {
                model_code: "MODEL-1".into(),
                protocol: AgentModelProtocol::ChatCompletions,
                base_url: "http://127.0.0.1:1/v1".into(),
                api_path: None,
                api_key: "sk-test".into(),
                model_name: "gpt-test".into(),
                thinking_enabled: false,
                reasoning_effort: None,
                responses_stateful_enabled: false,
                temperature: None,
                max_tokens: None,
                context_window_tokens: None,
                top_p: None,
                max_retries: None,
            },
            variables: Vec::new(),
            skills: Vec::new(),
            tools: Vec::new(),
            effective_tools: Vec::new(),
            tool_exposure: Default::default(),
            execution_scope: AgentExecutionScope::default(),
            limits: AgentRunLimits::default(),
            messages: vec![AgentMessage::user("run shell")],
            observations: Vec::new(),
            short_term_memory: None,
            started_at: "2026-06-05T00:00:00Z".into(),
        }
    }

    #[test]
    fn bash_exec_captures_command_output_and_status() {
        let manager = Arc::new(CommandTaskManager::default());
        let tool = BashExecTool(manager);
        let mut progress_lines = Vec::<String>::new();
        let result = tool
            .exec_with_progress(
                ToolRequest {
                    tool_name: "bash_exec".into(),
                    input: json!({
                        "command": "sh",
                        "args": ["-c", "printf 'one\\n'; printf 'two\\n' >&2"]
                    }),
                    invocation_id: None,
                },
                &test_context(),
                &mut |line| progress_lines.push(line),
            )
            .expect("bash command should complete");
        let output: serde_json::Value =
            serde_json::from_str(&result.observation.output).expect("output should be JSON");

        assert!(progress_lines.iter().any(|line| line == "one"));
        assert!(progress_lines.iter().any(|line| line == "[stderr] two"));
        assert_eq!(output["stdout"], json!("one\n"));
        assert_eq!(output["stderr"], json!("two\n"));
        assert_eq!(output["success"], json!(true));
    }

    #[cfg(unix)]
    #[test]
    fn bash_exec_accepts_a_complete_command_line_without_args() {
        let tool = BashExecTool(Arc::new(CommandTaskManager::default()));
        let result = tool
            .exec(
                ToolRequest {
                    tool_name: "bash_exec".into(),
                    input: json!({"command": "printf 'one' && printf ' two'"}),
                    invocation_id: None,
                },
                &test_context(),
            )
            .expect("complete command line should execute");
        let output: serde_json::Value = serde_json::from_str(&result.observation.output).unwrap();
        assert_eq!(output["stdout"], "one two");
        assert_eq!(output["success"], true);
    }

    #[cfg(unix)]
    #[test]
    fn long_bash_exec_returns_a_readable_and_stoppable_task() {
        let manager = Arc::new(CommandTaskManager::default());
        let context = test_context();
        let result = BashExecTool(manager.clone())
            .exec(
                ToolRequest {
                    tool_name: "bash_exec".into(),
                    input: json!({"command": "sleep 30"}),
                    invocation_id: None,
                },
                &context,
            )
            .expect("long command should move to background");
        let output: serde_json::Value = serde_json::from_str(&result.observation.output).unwrap();
        let task_id = output["taskId"]
            .as_str()
            .expect("long command should return a task id");
        assert_eq!(output["running"], true);
        assert_eq!(output["success"], serde_json::Value::Null);

        let read = BashTaskReadTool(manager.clone())
            .exec(
                ToolRequest {
                    tool_name: "bash_task_read".into(),
                    input: json!({"taskId": task_id}),
                    invocation_id: None,
                },
                &context,
            )
            .expect("background task should be readable");
        let read: serde_json::Value = serde_json::from_str(&read.observation.output).unwrap();
        assert_eq!(read["running"], true);

        let stopped = BashTaskStopTool(manager)
            .exec(
                ToolRequest {
                    tool_name: "bash_task_stop".into(),
                    input: json!({"taskId": task_id}),
                    invocation_id: None,
                },
                &context,
            )
            .expect("background task should be stoppable");
        let stopped: serde_json::Value = serde_json::from_str(&stopped.observation.output).unwrap();
        assert_eq!(stopped["stopped"], true);
    }
}
