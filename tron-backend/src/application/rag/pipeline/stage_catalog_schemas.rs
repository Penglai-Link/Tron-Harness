fn strategy_config_schema(strategy_id: &str) -> Value {
    match strategy_id {
        "PROJECT_DOC_RESOLVE_SOURCE" => json!({
            "type": "object",
            "properties": {
                "allowedUriSchemes": {
                    "type": "array",
                    "items": { "type": "string" },
                    "default": ["file", "http", "https"]
                },
                "maxBytes": {
                    "type": "integer",
                    "minimum": 1,
                    "default": 1048576
                }
            }
        }),
        "RAG_PARSE_MARKDOWN_DOCUMENT" => json!({
            "type": "object",
            "properties": {
                "format": {
                    "type": "string",
                    "enum": ["MARKDOWN"],
                    "default": "MARKDOWN"
                },
                "unsupportedFormatPolicy": {
                    "type": "string",
                    "enum": ["FAIL_FAST", "SKIP"],
                    "default": "FAIL_FAST"
                }
            }
        }),
        "RAG_PARSE_MINERU_DOCUMENT" => json!({
            "type": "object",
            "properties": {
                "supportedFormats": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "enum": ["PDF", "DOCX"]
                    },
                    "default": ["PDF", "DOCX"]
                },
                "unsupportedFormatPolicy": {
                    "type": "string",
                    "enum": ["FAIL_FAST", "SKIP"],
                    "default": "FAIL_FAST"
                }
            }
        }),
        "RAG_SPLIT_TEXT" => json!({
            "type": "object",
            "properties": {
                "splitter": {
                    "type": "object",
                    "properties": {
                        "type": {
                            "type": "string",
                            "enum": ["plainText", "markdownHeading"],
                            "default": "plainText"
                        },
                        "maxTokens": {
                            "type": "integer",
                            "minimum": 1,
                            "default": 800
                        },
                        "overlapTokens": {
                            "type": "integer",
                            "minimum": 0,
                            "default": 80
                        },
                        "headingDepth": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": 6,
                            "default": 6
                        },
                        "includeHeadingInIndexText": {
                            "type": "boolean",
                            "default": false
                        }
                    }
                }
            }
        }),
        "RAG_SPLIT_PLAIN_TEXT_V1" => json!({
            "type": "object",
            "properties": {
                "splitter": {
                    "type": "object",
                    "properties": {
                        "maxTokens": {
                            "type": "integer",
                            "minimum": 1,
                            "default": 800
                        },
                        "overlapTokens": {
                            "type": "integer",
                            "minimum": 0,
                            "default": 80
                        }
                    }
                }
            }
        }),
        "RAG_SPLIT_MARKDOWN_HEADING_V1" => json!({
            "type": "object",
            "properties": {
                "splitter": {
                    "type": "object",
                    "properties": {
                        "headingDepth": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": 6,
                            "default": 6
                        },
                        "includeHeadingInIndexText": {
                            "type": "boolean",
                            "default": true
                        },
                        "maxTokens": {
                            "type": "integer",
                            "minimum": 1,
                            "default": 800
                        },
                        "overlapTokens": {
                            "type": "integer",
                            "minimum": 0,
                            "default": 80
                        }
                    }
                }
            }
        }),
        _ => json!({
            "type": "object",
            "additionalProperties": false
        }),
    }
}
