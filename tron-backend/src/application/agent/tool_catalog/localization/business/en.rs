const COMMON: &[(&str, &str)] = include!("en/common.inc");
const DOMAIN: &[(&str, &str)] = include!("en/domain.inc");
const AUTOMATION: &[(&str, &str)] = include!("en/automation.inc");

pub(super) fn translate(key: &str) -> Option<&'static str> {
    [COMMON, DOMAIN, AUTOMATION]
        .into_iter()
        .flatten()
        .find_map(|(candidate, value)| (*candidate == key).then_some(*value))
}
