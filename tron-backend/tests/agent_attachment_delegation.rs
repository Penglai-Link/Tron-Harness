mod common;

use serde_json::{Value, json};
use std::io::Cursor;
use tron_backend::{
    app::AppState,
    application::resource::ChatSessionAttachmentCreateCommand,
    config::AppConfig,
    domain::agent::{AgentCollaborationOwner, AgentRunContext, ToolRequest},
};

fn invoke(
    state: &AppState,
    context: &AgentRunContext,
    name: &str,
    input: Value,
) -> Result<Value, String> {
    state
        .agent_tool_registry
        .execute_allowed(
            ToolRequest {
                tool_name: name.into(),
                input,
                invocation_id: None,
            },
            context,
        )
        .map_err(|error| error.to_string())
        .and_then(|result| {
            serde_json::from_str(&result.observation.output).map_err(|error| error.to_string())
        })
}

#[test]
fn delegated_tools_read_origin_attachments_without_copying_or_crossing_owner_workspace() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let state = AppState::new_with_id_provider(
        common::prepared_config(AppConfig {
            database_url: root.join("fixture.sqlite3").display().to_string(),
            ..Default::default()
        }),
        std::sync::Arc::new(tron_backend::shared::id::FixedSnowflakeIdProvider::new(
            801_500_000_000_000_000,
        )),
    )
    .unwrap();
    let mut parent: AgentRunContext = serde_json::from_value(json!({
        "agentCode":"main-agent", "runCode":eliteforge_agent_core::RunId::new_v4().to_string(), "conversationCode":"PARENT", "workspaceCode":"TRON_CORE", "userCode":"USER", "locale":"en-US", "agentKind":"REACT",
        "model":{"modelCode":"fixture", "protocol":"CHAT_COMPLETIONS", "baseUrl":"http://127.0.0.1:1/v1", "apiKey":"synthetic", "modelName":"fixture"},
        "variables":[], "skills":[], "tools":[], "executionScope":{"localDirectory":root,"allowedLocalDirectories":[root]}, "limits":{"maxSteps":12,"maxToolCalls":12}, "messages":[], "observations":[], "startedAt":"fixture"
    })).unwrap();
    parent.tools =
        tron_backend::application::agent::BusinessToolCatalog::list_tools(None, None, "en-US")
            .into_iter()
            .map(tron_backend::application::agent::tool_record_to_effective_tool)
            .collect();
    invoke(
        &state,
        &parent,
        "word_create",
        json!({"path":"fixture.docx","paragraphs":[{"text":"Synthetic attachment fixture"}]}),
    )
    .unwrap();
    for (code, name, mime, bytes) in [
        (
            "DOC",
            "fixture.docx",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            std::fs::read(root.join("fixture.docx")).unwrap(),
        ),
        (
            "TEXT",
            "fixture.txt",
            "text/plain",
            b"synthetic text fixture".to_vec(),
        ),
    ] {
        state
            .resources
            .store_chat_session_attachment(
                ChatSessionAttachmentCreateCommand {
                    workspace_code: parent.workspace_code.clone(),
                    session_code: "PARENT".into(),
                    attachment_code: code.into(),
                    file_name: name.into(),
                    content_type: mime.into(),
                },
                &mut Cursor::new(bytes),
            )
            .unwrap();
    }
    let mut child = parent.clone();
    child.agent_code = Some("reviewer".into());
    child.conversation_code = Some("CHILD".into());
    child.collaboration_owner = Some(AgentCollaborationOwner {
        kind: "USER".into(),
        code: "USER".into(),
        origin_session_code: Some("PARENT".into()),
        task_code: None,
    });
    let word = json!({"fileRef":{"kind":"session_attachment","attachmentCode":"DOC"}});
    let text = json!({"attachmentCode":"TEXT"});
    assert_eq!(
        invoke(&state, &child, "word_read", word.clone()).unwrap()["paragraphs"][0]["text"],
        "Synthetic attachment fixture"
    );
    assert_eq!(
        invoke(&state, &child, "chat_attachment_read_text", text.clone()).unwrap()["content"],
        "synthetic text fixture"
    );
    // Nested delegation keeps the same trusted origin instead of the immediate child's empty session.
    let mut grandchild = child.clone();
    grandchild.conversation_code = Some("GRANDCHILD".into());
    assert!(invoke(&state, &grandchild, "word_read", word.clone()).is_ok());
    assert!(
        state
            .resources
            .chat_session_attachment_records("CHILD")
            .unwrap()
            .is_empty()
    );
    assert!(
        state
            .resources
            .chat_session_attachment_records("GRANDCHILD")
            .unwrap()
            .is_empty()
    );
    for changed in 0..4 {
        let mut foreign = child.clone();
        match changed {
            0 => foreign.workspace_code = "OTHER-WORKSPACE".into(),
            1 => foreign.user_code = Some("OTHER-USER".into()),
            2 => foreign.collaboration_owner = None,
            _ => {
                foreign
                    .collaboration_owner
                    .as_mut()
                    .unwrap()
                    .origin_session_code = Some("UNRELATED".into())
            }
        }
        assert!(invoke(&state, &foreign, "word_read", word.clone()).is_err());
        assert!(invoke(&state, &foreign, "chat_attachment_read_text", text.clone()).is_err());
    }
    assert!(invoke(&state,&child,"word_read",json!({"fileRef":{"kind":"session_attachment","attachmentCode":"DOC","sessionCode":"PARENT"}})).is_err());
}
