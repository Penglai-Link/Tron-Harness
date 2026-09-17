fn resource_kind_schema() -> serde_json::Value {
    json!({ "type": "string", "enum": ["File", "Image", "Pdf", "MysqlConnection", "PostgresqlConnection", "RedisConnection", "KafkaConnection", "ServerConnection", "ServerAccount"] })
}

fn resource_file_kind_schema() -> serde_json::Value {
    json!({ "type": "string", "enum": ["File", "Image", "Pdf"] })
}

fn resource_data_source_kind_schema() -> serde_json::Value {
    json!({ "type": "string", "enum": ["MysqlConnection", "PostgresqlConnection", "RedisConnection", "KafkaConnection"] })
}

fn resource_data_source_kind_attribute_branches() -> Vec<Value> {
    [
        ("MysqlConnection", mysql_connection_attributes_schema()),
        (
            "PostgresqlConnection",
            postgresql_connection_attributes_schema(),
        ),
        ("RedisConnection", redis_connection_attributes_schema()),
        ("KafkaConnection", kafka_connection_attributes_schema()),
    ]
    .into_iter()
    .map(|(kind, attributes)| {
        json!({
            "properties": {
                "kind": { "const": kind },
                "attributes": attributes,
            },
            "required": ["kind", "attributes"]
        })
    })
    .collect()
}

fn resource_data_source_attributes_schema() -> Value {
    json!({
        "type": "object",
        "oneOf": [
            mysql_connection_attributes_schema(),
            postgresql_connection_attributes_schema(),
            redis_connection_attributes_schema(),
            kafka_connection_attributes_schema()
        ],
        "x-i18n-description-key": "agent.toolCatalog.schemas.resource.dataSource.attributes.description"
    })
}

fn mysql_connection_attributes_schema() -> Value {
    json!({
        "title": "MysqlConnection",
        "type": "object",
        "required": ["connectionMode", "endpoints", "database", "credentials", "tls"],
        "properties": {
            "connectionMode": { "type": "string", "enum": ["SINGLE", "FAILOVER", "LOAD_BALANCE", "REPLICATION"] },
            "endpoints": {
                "type": "array", "minItems": 1, "maxItems": 128,
                "items": connection_endpoint_schema(true)
            },
            "database": { "type": "string", "minLength": 1, "maxLength": 64 },
            "credentials": credentials_schema(true),
            "tls": tls_schema(&["DISABLED", "PREFERRED", "REQUIRED", "VERIFY_CA", "VERIFY_IDENTITY"], false),
            "properties": scalar_properties_schema()
        },
        "additionalProperties": false
    })
}

fn postgresql_connection_attributes_schema() -> Value {
    json!({
        "title": "PostgresqlConnection",
        "type": "object",
        "required": ["endpoints", "database", "credentials", "targetSession", "loadBalance", "tls"],
        "properties": {
            "endpoints": {
                "type": "array", "minItems": 1, "maxItems": 128,
                "items": connection_endpoint_schema(false)
            },
            "database": { "type": "string", "minLength": 1, "maxLength": 63 },
            "credentials": credentials_schema(true),
            "targetSession": { "type": "string", "enum": ["ANY", "READ_WRITE", "READ_ONLY", "PRIMARY", "STANDBY", "PREFER_STANDBY"] },
            "loadBalance": { "type": "string", "enum": ["DISABLED", "RANDOM"] },
            "tls": tls_schema(&["DISABLED", "ALLOW", "PREFERRED", "REQUIRED", "VERIFY_CA", "VERIFY_FULL"], true),
            "properties": scalar_properties_schema()
        },
        "additionalProperties": false
    })
}

fn redis_connection_attributes_schema() -> Value {
    json!({
        "title": "RedisConnection",
        "type": "object",
        "required": ["topology", "endpoints", "database", "tls"],
        "properties": {
            "topology": { "type": "string", "enum": ["STANDALONE", "SENTINEL", "CLUSTER"] },
            "endpoints": {
                "type": "array", "minItems": 1, "maxItems": 128,
                "items": connection_endpoint_schema(false)
            },
            "database": { "type": "integer", "minimum": 0 },
            "credentials": credentials_schema(false),
            "tls": tls_schema(&["DISABLED", "REQUIRED", "VERIFY_CA", "VERIFY_FULL"], true),
            "sentinel": {
                "type": "object", "required": ["masterName", "tls"],
                "properties": {
                    "masterName": { "type": "string", "minLength": 1, "maxLength": 128, "pattern": "^\\S+$" },
                    "credentials": credentials_schema(false),
                    "tls": tls_schema(&["DISABLED", "REQUIRED", "VERIFY_CA", "VERIFY_FULL"], true)
                },
                "additionalProperties": false
            },
            "properties": scalar_properties_schema()
        },
        "allOf": [{ "oneOf": [
            { "properties": { "topology": { "const": "SENTINEL" } }, "required": ["sentinel"] },
            { "properties": { "topology": { "enum": ["STANDALONE", "CLUSTER"] } }, "not": { "required": ["sentinel"] } }
        ] }],
        "additionalProperties": false
    })
}

fn kafka_connection_attributes_schema() -> Value {
    json!({
        "title": "KafkaConnection",
        "type": "object",
        "required": ["brokers", "securityProtocol"],
        "properties": {
            "brokers": {
                "type": "array", "minItems": 1, "maxItems": 128,
                "items": connection_endpoint_schema(false)
            },
            "securityProtocol": { "type": "string", "enum": ["PLAINTEXT", "SSL", "SASL_PLAINTEXT", "SASL_SSL"] },
            "sasl": {
                "type": "object", "required": ["mechanism", "username", "password"],
                "properties": {
                    "mechanism": { "type": "string", "enum": ["PLAIN", "SCRAM_SHA_256", "SCRAM_SHA_512"] },
                    "username": { "type": "string", "minLength": 1, "maxLength": 128 },
                    "password": secret_reference_schema()
                },
                "additionalProperties": false
            },
            "tls": kafka_tls_schema()
        },
        "allOf": [
            { "oneOf": [
                { "properties": { "securityProtocol": { "enum": ["SASL_PLAINTEXT", "SASL_SSL"] } }, "required": ["sasl"] },
                { "properties": { "securityProtocol": { "enum": ["PLAINTEXT", "SSL"] } }, "not": { "required": ["sasl"] } }
            ] },
            { "oneOf": [
                { "properties": { "securityProtocol": { "enum": ["SSL", "SASL_SSL"] } }, "required": ["tls"] },
                { "properties": { "securityProtocol": { "enum": ["PLAINTEXT", "SASL_PLAINTEXT"] } }, "not": { "required": ["tls"] } }
            ] }
        ],
        "additionalProperties": false
    })
}

fn connection_endpoint_schema(include_role: bool) -> Value {
    let mut properties = Map::new();
    properties.insert(
        "host".to_string(),
        json!({ "type": "string", "minLength": 1, "maxLength": 255 }),
    );
    properties.insert(
        "port".to_string(),
        json!({ "type": "integer", "minimum": 1, "maximum": 65535 }),
    );
    if include_role {
        properties.insert(
            "role".to_string(),
            json!({ "type": "string", "enum": ["PRIMARY", "REPLICA"] }),
        );
    }
    json!({
        "type": "object",
        "required": ["host", "port"],
        "properties": properties,
        "additionalProperties": false
    })
}

fn credentials_schema(username_required: bool) -> Value {
    let mut required = vec![Value::String("password".to_string())];
    if username_required {
        required.push(Value::String("username".to_string()));
    }
    json!({
        "type": "object",
        "required": required,
        "properties": {
            "username": { "type": "string", "minLength": 1, "maxLength": 128 },
            "password": secret_reference_schema()
        },
        "additionalProperties": false
    })
}

fn secret_reference_schema() -> Value {
    json!({
        "oneOf": [
            { "type": "string", "minLength": 1, "maxLength": 128, "description": "Legacy variable key" },
            {
                "type": "object",
                "oneOf": [
                    {
                        "properties": {
                            "source": { "const": "DIRECT" },
                            "value": { "type": "string", "minLength": 1, "maxLength": 4096 }
                        },
                        "required": ["source", "value"],
                        "additionalProperties": false
                    },
                    {
                        "properties": {
                            "source": { "const": "VARIABLE" },
                            "variableKey": { "type": "string", "minLength": 1, "maxLength": 128 }
                        },
                        "required": ["source", "variableKey"],
                        "additionalProperties": false
                    }
                ]
            }
        ]
    })
}

fn tls_schema(modes: &[&str], enforce_client_key_dependencies: bool) -> Value {
    let mut schema = json!({
        "type": "object",
        "required": ["mode"],
        "properties": {
            "mode": { "type": "string", "enum": modes },
            "caCertificate": { "type": "string", "minLength": 1, "maxLength": 128 },
            "clientCertificate": { "type": "string", "minLength": 1, "maxLength": 128 },
            "clientKey": { "type": "string", "minLength": 1, "maxLength": 128 },
            "clientKeyPassword": { "type": "string", "minLength": 1, "maxLength": 128 }
        },
        "additionalProperties": false
    });
    if enforce_client_key_dependencies {
        schema["allOf"] = json!([
            {
                "oneOf": [
                    { "required": ["clientCertificate", "clientKey"] },
                    { "not": { "anyOf": [{ "required": ["clientCertificate"] }, { "required": ["clientKey"] }] } }
                ]
            },
            { "oneOf": [
                { "required": ["clientKeyPassword", "clientKey"] },
                { "not": { "required": ["clientKeyPassword"] } }
            ] }
        ]);
    }
    schema
}

fn kafka_tls_schema() -> Value {
    let mut schema = tls_schema(&["REQUIRED", "VERIFY_CA", "VERIFY_FULL"], true);
    schema["properties"]
        .as_object_mut()
        .expect("TLS schema properties are an object")
        .remove("clientKeyPassword");
    schema["allOf"]
        .as_array_mut()
        .expect("TLS schema dependencies are an array")
        .pop();
    schema
}

fn scalar_properties_schema() -> Value {
    json!({
        "type": "object",
        "maxProperties": 128,
        "additionalProperties": {
            "oneOf": [
                { "type": "string" },
                { "type": "number" },
                { "type": "boolean" }
            ]
        }
    })
}

fn resource_server_account_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "required": ["accountCode", "name", "username", "authenticationType", "allowInheritance"],
        "properties": {
            "accountCode": resource_code_schema(),
            "name": { "type": "string", "minLength": 1 },
            "description": { "type": "string" },
            "username": { "type": "string", "minLength": 1 },
            "authenticationType": { "type": "string", "enum": ["Password", "PrivateKey"] },
            "passwordVariableKey": { "type": "string", "minLength": 1 },
            "privateKeyVariableKey": { "type": "string", "minLength": 1 },
            "privateKeyPassphraseVariableKey": { "type": "string", "minLength": 1 },
            "allowInheritance": { "type": "boolean" }
        },
        "allOf": [{ "oneOf": [
            { "properties": { "authenticationType": { "const": "Password" } }, "required": ["passwordVariableKey"] },
            { "properties": { "authenticationType": { "const": "PrivateKey" } }, "required": ["privateKeyVariableKey"] }
        ] }],
        "additionalProperties": false
    })
}

fn resource_workspace_code_schema() -> serde_json::Value {
    json!({ "type": "string", "minLength": 1 })
}

fn resource_code_schema() -> serde_json::Value {
    json!({ "type": "string", "minLength": 1, "maxLength": 64 })
}
