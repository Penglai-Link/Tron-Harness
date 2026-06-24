use serde_json::json;

use crate::{
    domain::{
        agent::AgentRunContext,
        short_term_memory::{ShortTermMemoryError, ShortTermMemoryTodo, TodoPriority, TodoStatus},
    },
    shared::time::current_timestamp,
};

use super::{ShortTermMemoryService, TodoUpdateItem};

const MAX_TODO_CONTENT_CHARS: usize = 500;
const MAX_TODOS: usize = 50;

impl ShortTermMemoryService {
    pub fn replace_todos(
        &self,
        context: &AgentRunContext,
        items: Vec<TodoUpdateItem>,
    ) -> Result<Vec<ShortTermMemoryTodo>, ShortTermMemoryError> {
        let memory = context
            .short_term_memory
            .as_ref()
            .ok_or(ShortTermMemoryError::TodoInvalid)?;
        if !memory.todo_enabled {
            return Err(ShortTermMemoryError::TodoInvalid);
        }
        let todos = self.validate_todos(context, items)?;
        let event = self.new_event(
            &memory.session_code,
            &context.workspace_code,
            Some(&context.run_code),
            Some(&memory.input_code),
            None,
            None,
            "agent.todo.updated",
            "TOOL",
            json!({"todos": todos}),
            None,
        )?;
        let mut repository = self.repository();
        if let Some(event) = event {
            repository.replace_todos_with_event(
                &memory.session_code,
                &context.workspace_code,
                &todos,
                &event,
            )?;
        } else {
            repository.replace_todos(&memory.session_code, &context.workspace_code, &todos)?;
        }
        Ok(todos)
    }

    pub fn list_todos(
        &self,
        session_code: &str,
    ) -> Result<Vec<ShortTermMemoryTodo>, ShortTermMemoryError> {
        self.ensure_session(session_code)?;
        self.repository().list_todos(session_code)
    }

    fn validate_todos(
        &self,
        context: &AgentRunContext,
        items: Vec<TodoUpdateItem>,
    ) -> Result<Vec<ShortTermMemoryTodo>, ShortTermMemoryError> {
        if items.len() > MAX_TODOS {
            return Err(ShortTermMemoryError::TodoInvalid);
        }
        let mut in_progress = 0usize;
        let mut positions = std::collections::BTreeSet::new();
        let now = current_timestamp();
        let memory = context
            .short_term_memory
            .as_ref()
            .ok_or(ShortTermMemoryError::TodoInvalid)?;
        if !memory.todo_enabled {
            return Err(ShortTermMemoryError::TodoInvalid);
        }
        items
            .into_iter()
            .map(|item| {
                if !positions.insert(item.position) || item.position < 0 {
                    return Err(ShortTermMemoryError::TodoInvalid);
                }
                let content = item.content.trim().to_string();
                if content.is_empty() || content.chars().count() > MAX_TODO_CONTENT_CHARS {
                    return Err(ShortTermMemoryError::TodoInvalid);
                }
                let status = TodoStatus::from_code(item.status.as_str())?;
                if status == TodoStatus::InProgress {
                    in_progress += 1;
                }
                if in_progress > 1 {
                    return Err(ShortTermMemoryError::TodoInvalid);
                }
                Ok(ShortTermMemoryTodo {
                    id: 0,
                    todo_code: item.todo_code.unwrap_or(self.next_code("TODO")?),
                    session_code: memory.session_code.clone(),
                    workspace_code: context.workspace_code.clone(),
                    position: item.position,
                    content,
                    status,
                    priority: TodoPriority::from_code(item.priority.as_str())?,
                    created_by_run_code: Some(context.run_code.clone()),
                    updated_by_run_code: Some(context.run_code.clone()),
                    created_at: now.clone(),
                    updated_at: now.clone(),
                })
            })
            .collect()
    }
}
