include!("service/foundation.inc");
mod knowledge_quality;
pub mod management;
include!("service/sessions.inc");
include!("service/messages.inc");
include!("service/artifacts.inc");
include!("service/validation.inc");

mod workspace_cleanup;

include!("service/application_context.inc");

include!("service/composer_input.inc");

mod history_deletion;
