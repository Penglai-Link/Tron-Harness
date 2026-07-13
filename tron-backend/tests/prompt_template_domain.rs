use std::collections::HashMap;

use serde_json::json;
use tron_backend::domain::prompt::{
    PromptVariable, PromptVariableType, TemplateError, analyze_template, render_template,
    render_typed_template,
};

#[test]
fn template_analysis_collects_valid_variables_and_ignores_escaped_placeholders() {
    let variables = analyze_template(r"Hello {{name}} / \{{literal}} / {{_trace_2}} / {{name}}")
        .expect("valid prompt template should be analyzed");

    assert_eq!(variables, vec!["name", "_trace_2"]);
}

#[test]
fn template_rendering_is_non_recursive_and_unescapes_literal_placeholders() {
    let values = HashMap::from([
        ("name".to_string(), "{{nested}}".to_string()),
        ("_trace_2".to_string(), "trace-2".to_string()),
    ]);

    let rendered = render_template(r"Hello {{name}} / \{{literal}} / {{_trace_2}}", &values)
        .expect("complete prompt variables should render");

    assert_eq!(rendered, "Hello {{nested}} / {{literal}} / trace-2");
}

#[test]
fn template_analysis_rejects_invalid_variable_names() {
    let error = analyze_template("Hello {{invalid-name}}")
        .expect_err("invalid variable names must be rejected while saving");

    assert_eq!(
        error,
        TemplateError::InvalidVariableName("invalid-name".to_string())
    );
}

#[test]
fn template_rendering_rejects_missing_and_unknown_variables_before_generation() {
    let missing = render_template(
        "Hello {{name}}",
        &HashMap::from([("unknown".to_string(), "value".to_string())]),
    )
    .expect_err("missing variables must fail before runtime rendering");
    assert_eq!(
        missing,
        TemplateError::MissingVariables(vec!["name".to_string()])
    );

    let unknown = render_template(
        "Hello {{name}}",
        &HashMap::from([
            ("name".to_string(), "Tron".to_string()),
            ("unknown".to_string(), "value".to_string()),
        ]),
    )
    .expect_err("unknown variables must fail before preview generation");
    assert_eq!(
        unknown,
        TemplateError::UnknownVariables(vec!["unknown".to_string()])
    );
}

#[test]
fn typed_template_rendering_validates_and_serializes_declared_types() {
    let variables = vec![
        PromptVariable {
            name: "name".to_string(),
            variable_type: PromptVariableType::String,
            description: "Display name".to_string(),
        },
        PromptVariable {
            name: "attempts".to_string(),
            variable_type: PromptVariableType::Number,
            description: "Retry count".to_string(),
        },
        PromptVariable {
            name: "enabled".to_string(),
            variable_type: PromptVariableType::Boolean,
            description: "Feature flag".to_string(),
        },
        PromptVariable {
            name: "options".to_string(),
            variable_type: PromptVariableType::Json,
            description: "Runtime options".to_string(),
        },
    ];
    let values = HashMap::from([
        ("name".to_string(), json!("Ada")),
        ("attempts".to_string(), json!(3)),
        ("enabled".to_string(), json!(true)),
        ("options".to_string(), json!({ "mode": "safe" })),
    ]);

    assert_eq!(
        render_typed_template(
            "{{name}}/{{attempts}}/{{enabled}}/{{options}}",
            &variables,
            &values,
        ),
        Ok("Ada/3/true/{\"mode\":\"safe\"}".to_string())
    );

    let invalid_values = HashMap::from([
        ("name".to_string(), json!("Ada")),
        ("attempts".to_string(), json!("three")),
        ("enabled".to_string(), json!(true)),
        ("options".to_string(), json!({ "mode": "safe" })),
    ]);
    assert_eq!(
        render_typed_template(
            "{{name}}/{{attempts}}/{{enabled}}/{{options}}",
            &variables,
            &invalid_values,
        ),
        Err(TemplateError::VariableTypeMismatch {
            variable_name: "attempts".to_string(),
            expected_type: PromptVariableType::Number,
        })
    );
}
