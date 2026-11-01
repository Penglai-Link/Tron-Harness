// Exercise upstream tool contracts using Tron's resolved dependency graph.
// Upstream feature-gated tests are not enabled by this wrapper; web and media
// behavior is covered separately by framework_web_media_tools.
#![allow(unexpected_cfgs)]
#[path = "../../.agent-deps/agent-framework/crates/capability/eliteforge-agent-capability-tools/tests/capability_packs.rs"]
mod framework_capability_packs;
