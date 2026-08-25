fn required_actor(context: &AgentRunContext) -> Result<&str, ToolError> {
    context
        .user_code
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            ToolError::ExecutionFailed("resource tools require an authenticated user".to_string())
        })
}

fn validate_resource_binary_input(input: &ResourceBinaryInput) -> Result<(), ToolError> {
    match input.action {
        ResourceBinaryAction::Upload => {
            if input.source_path.is_none() || input.target_path.is_some() {
                return Err(invalid_input("upload requires sourcePath only"));
            }
        }
        ResourceBinaryAction::Download => {
            if input.target_path.is_none() || input.source_path.is_some() {
                return Err(invalid_input("download requires targetPath only"));
            }
        }
    }
    Ok(())
}

fn invalid_input(message: &str) -> ToolError {
    ToolError::ExecutionFailed(format!("resource tool input is invalid: {message}"))
}

fn run_resource_future<F, T>(future: F) -> Result<T, ToolError>
where
    F: Future<Output = Result<T, ResourceError>> + Send + 'static,
    T: Send + 'static,
{
    thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| ToolError::ExecutionFailed(error.to_string()))?
            .block_on(future)
            .map_err(service_tool_error)
    })
    .join()
    .map_err(|_| ToolError::ExecutionFailed("resource tool worker panicked".into()))?
}
