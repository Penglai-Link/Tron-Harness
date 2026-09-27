include!("runtime/foundation.inc");
include!("runtime/execution.inc");

#[path = "runtime/framework_model.rs"]
mod framework_model;

#[path = "runtime/framework_routing.rs"]
mod framework_routing;
pub(crate) use runtime_compaction::CompactionModelInput;
