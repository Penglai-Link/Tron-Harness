//! Product naming and localization for the official structured-question tool.
use crate::application::agent::tool_catalog::{materialize_schema, translate};
use crate::domain::agent::EffectiveTool;
use crate::shared::locale::SupportedLocale;
pub const CHAT_ASK_USER_TOOL: &str = "chat_ask_user";
pub fn ask_user_definition(locale: &str) -> EffectiveTool {
    let locale = SupportedLocale::from_raw(locale);
    let mut schema =
        eliteforge_agent_official_plugin_tools_human::AskUserTool::descriptor().input_schema;
    // Tron offers both free-text and skip on every question. Omit optional
    // configuration that its presentation does not expose; shared defaults apply.
    let question = &mut schema["properties"]["questions"]["items"]["properties"];
    if let Some(properties) = question.as_object_mut() {
        properties.remove("allow_free_text");
        properties.remove("allow_skip");
    }
    for (pointer, name) in [
        ("/properties/intro", "intro"),
        ("/properties/questions", "questions"),
        (
            "/properties/questions/items/properties/question",
            "question",
        ),
        ("/properties/questions/items/properties/options", "options"),
        (
            "/properties/questions/items/properties/options/items/properties/label",
            "optionLabel",
        ),
        (
            "/properties/questions/items/properties/options/items/properties/description",
            "optionDescription",
        ),
    ] {
        if let Some(field) = schema.pointer_mut(pointer) {
            field["x-i18n-description-key"] =
                format!("agent.toolCatalog.tools.chat.ask_user.params.{name}").into();
        }
    }
    EffectiveTool {
        tool_name: CHAT_ASK_USER_TOOL.into(),
        domain_code: Some("chat".into()),
        description: translate(locale, "agent.toolCatalog.tools.chat.ask_user.description").into(),
        input_schema: materialize_schema(locale, &schema),
        skill_name: None,
        required_variable_keys: vec![],
    }
}
