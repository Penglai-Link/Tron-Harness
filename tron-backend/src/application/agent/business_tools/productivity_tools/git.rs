use super::*;
async fn git(
    directory: &Path,
    args: &[&str],
    control: Option<&dyn ToolExecutionControl>,
) -> Result<Value, ToolError> {
    run(
        "git",
        &args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>(),
        directory,
        30,
        control,
    )
    .await
}
async fn checked(
    directory: &Path,
    args: &[&str],
    control: Option<&dyn ToolExecutionControl>,
) -> Result<String, ToolError> {
    let result = git(directory, args, control).await?;
    if result["exitCode"] != 0 {
        return Err(error(result["stderr"].as_str().unwrap_or("git failed")));
    }
    if result["truncated"] == true {
        return Err(error("Git output exceeds limit; narrow the query"));
    }
    Ok(result["stdout"]
        .as_str()
        .unwrap_or_default()
        .trim_end_matches(['\n', '\r'])
        .to_string())
}
async fn worktrees(
    directory: &Path,
    control: Option<&dyn ToolExecutionControl>,
) -> Result<Value, ToolError> {
    let output = checked(
        directory,
        &["worktree", "list", "--porcelain", "-z"],
        control,
    )
    .await?;
    let mut items = Vec::new();
    let mut current = json!({});
    for field in output.split('\0') {
        if let Some(path) = field.strip_prefix("worktree ") {
            if current.get("path").is_some() {
                items.push(current);
            }
            current = json!({"path":path,"branch":null,"head":null});
        } else if let Some(branch) = field.strip_prefix("branch refs/heads/") {
            current["branch"] = json!(branch);
        } else if let Some(head) = field.strip_prefix("HEAD ") {
            current["head"] = json!(head);
        }
    }
    if current.get("path").is_some() {
        items.push(current);
    }
    Ok(json!(items))
}
pub(super) async fn execute(
    name: &str,
    input: &Value,
    context: &AgentRunContext,
    control: Option<&dyn ToolExecutionControl>,
) -> Result<Value, ToolError> {
    let directory = cwd(input, context)?;
    checked(&directory, &["rev-parse", "--show-toplevel"], control).await?;
    match name {
        "git_status" => git(&directory, &["status", "--short", "--branch"], control).await,
        "git_diff" => {
            let mut args = vec!["diff", "--no-ext-diff", "--no-textconv"];
            if input["staged"] == true {
                args.push("--cached");
            }
            args.push("--");
            if let Some(file) = input.get("path").and_then(Value::as_str) {
                args.push(file);
            }
            git(&directory, &args, control).await
        }
        "git_log" => {
            let limit = input["limit"]
                .as_u64()
                .unwrap_or(20)
                .clamp(1, 100)
                .to_string();
            git(
                &directory,
                &["log", "--oneline", "-n", &limit, "--"],
                control,
            )
            .await
        }
        "git_switch_branch" => {
            let branch = text(input, "branch")?;
            if branch.starts_with('-') {
                return Err(error("invalid branch"));
            }
            checked(
                &directory,
                &["check-ref-format", "--branch", branch],
                control,
            )
            .await?;
            let args = if input["create"] == true {
                vec!["switch", "-c", branch]
            } else {
                vec!["switch", branch]
            };
            checked(&directory, &args, control).await?;
            Ok(json!({"cwd":directory,"branch":branch,"switched":true}))
        }
        "worktree_list" => Ok(json!({"worktrees":worktrees(&directory, control).await?})),
        "worktree_select" => {
            let selected = PathBuf::from(text(input, "path")?)
                .canonicalize()
                .map_err(error)?;
            let items = worktrees(&directory, control).await?;
            let registered = items.as_array().is_some_and(|items| {
                items.iter().any(|item| {
                    item["path"]
                        .as_str()
                        .and_then(|path| Path::new(path).canonicalize().ok())
                        .as_ref()
                        == Some(&selected)
                })
            });
            if !registered {
                return Err(error(
                    "path is not a registered worktree of the current repository",
                ));
            }
            Ok(json!({"cwd":selected,"selected":true}))
        }
        "worktree_create" => {
            let name = text(input, "name")?;
            if name.len() > 64
                || name.starts_with('.')
                || !name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                return Err(error(
                    "worktree name must contain 1-64 letters, digits, underscores or hyphens",
                ));
            }
            let branch = text(input, "branch")?;
            if branch.starts_with('-') {
                return Err(error("invalid branch"));
            }
            checked(
                &directory,
                &["check-ref-format", "--branch", branch],
                control,
            )
            .await?;
            let reference = input
                .get("startPoint")
                .and_then(Value::as_str)
                .unwrap_or("HEAD");
            let commit = checked(
                &directory,
                &[
                    "rev-parse",
                    "--verify",
                    "--end-of-options",
                    &format!("{reference}^{{commit}}"),
                ],
                control,
            )
            .await?;
            let items = worktrees(&directory, control).await?;
            let root = PathBuf::from(
                items
                    .as_array()
                    .and_then(|items| items.first())
                    .and_then(|item| item["path"].as_str())
                    .ok_or_else(|| error("main worktree unavailable"))?,
            );
            let parent = root
                .parent()
                .ok_or_else(|| error("repository parent unavailable"))?
                .canonicalize()
                .map_err(error)?;
            let base = parent.join(".tron-worktrees");
            if base.exists() && base.canonicalize().map_err(error)? != base {
                return Err(error("managed worktree root must not be a symlink"));
            }
            if let Some(control) = control {
                control.check_active()?;
            }
            std::fs::create_dir_all(&base).map_err(error)?;
            let destination = base.join(name);
            if destination.exists() {
                return Err(error("worktree destination already exists"));
            }
            checked(
                &directory,
                &[
                    "worktree",
                    "add",
                    "-b",
                    branch,
                    "--",
                    destination.to_str().ok_or_else(|| error("invalid path"))?,
                    &commit,
                ],
                control,
            )
            .await?;
            // Creation is explicit and activates the new checkout for the next model boundary.
            Ok(json!({"cwd":destination,"branch":branch,"created":true,"selected":true}))
        }
        _ => Err(error("unknown development tool")),
    }
}
