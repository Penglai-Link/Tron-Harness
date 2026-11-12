[package]
name = "__PACK_SLUG__-backend"
version = "__PACK_VERSION__"
edition = "2024"
publish = false

[[bin]]
name = "capability-backend"
path = "src/main.rs"

[dependencies]
async-trait = "0.1"
serde_json = "1"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
eliteforge-agent-core = { git = "https://gitee.com/person-ai/eliteforge-agent-framework.git", tag = "1.0.0" }
eliteforge-agent-capability-runtime = { git = "https://gitee.com/person-ai/eliteforge-agent-framework.git", tag = "1.0.0" }

[profile.release]
strip = "symbols"
