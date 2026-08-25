const COMMON: &[(&str, &str)] = include!("zh/common.inc");
const DOMAIN: &[(&str, &str)] = include!("zh/domain.inc");
const AUTOMATION: &[(&str, &str)] = include!("zh/automation.inc");

pub(super) fn translate(key: &str) -> Option<&'static str> {
    [COMMON, DOMAIN, AUTOMATION]
        .into_iter()
        .flatten()
        .find_map(|(candidate, value)| (*candidate == key).then_some(*value))
}
