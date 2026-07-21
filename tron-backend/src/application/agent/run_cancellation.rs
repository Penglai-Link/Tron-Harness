use std::{sync::Arc, time::Duration};

use super::AgentRuntime;

const RUNTIME_CANCEL_TIMEOUT: Duration = Duration::from_secs(5);

pub(super) async fn cancel_runtime_after_stop(runtime: &Arc<dyn AgentRuntime>, run_code: &str) {
    if !runtime.capabilities().supports_cancel {
        return;
    }
    let diagnostic =
        match tokio::time::timeout(RUNTIME_CANCEL_TIMEOUT, runtime.cancel(run_code)).await {
            Ok(Ok(())) => return,
            Ok(Err(error)) => format!(
                "runtime adapter `{}` cancel failed for run `{run_code}`: {}",
                runtime.key().code(),
                error.message(),
            ),
            Err(_) => format!(
                "runtime adapter `{}` cancel timed out for run `{run_code}`",
                runtime.key().code(),
            ),
        };
    eprintln!("{diagnostic}");
}
