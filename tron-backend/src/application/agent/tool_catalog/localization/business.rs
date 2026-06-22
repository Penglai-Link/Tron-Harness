mod en;
mod zh;

pub(super) fn translate_en_us(key: &str) -> Option<&'static str> {
    en::EN_US
        .iter()
        .find_map(|(candidate, value)| (*candidate == key).then_some(*value))
}

pub(super) fn translate_zh_cn(key: &str) -> Option<&'static str> {
    zh::ZH_CN
        .iter()
        .find_map(|(candidate, value)| (*candidate == key).then_some(*value))
}
