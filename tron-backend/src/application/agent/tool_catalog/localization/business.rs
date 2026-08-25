mod en;
mod zh;

pub(super) fn translate_en_us(key: &str) -> Option<&'static str> {
    en::translate(key)
}

pub(super) fn translate_zh_cn(key: &str) -> Option<&'static str> {
    zh::translate(key)
}
