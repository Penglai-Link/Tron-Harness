use serde::{Deserialize, Deserializer, de::Error};

#[derive(Deserialize)]
#[serde(untagged)]
enum PercentageOrBasisPoints {
    Percentage(String),
    BasisPoints(u32),
}

pub(super) fn optional_ratio<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<u32>, D::Error> {
    optional_basis_points(deserializer, true)
}

fn optional_basis_points<'de, D: Deserializer<'de>>(
    deserializer: D,
    bounded_ratio: bool,
) -> Result<Option<u32>, D::Error> {
    Option::<PercentageOrBasisPoints>::deserialize(deserializer)?
        .map(|value| match value {
            // Integer values are expressed in basis points.
            PercentageOrBasisPoints::BasisPoints(value) => Ok(value),
            PercentageOrBasisPoints::Percentage(value) => {
                let points = parse_percentage(&value).map_err(D::Error::custom)?;
                if bounded_ratio && points > 10_000 {
                    return Err(D::Error::custom(
                        "percentage ratio must be between 0% and 100%",
                    ));
                }
                Ok(points)
            }
        })
        .transpose()
}

fn parse_percentage(value: &str) -> Result<u32, &'static str> {
    const INVALID: &str =
        "expected an unsigned percentage such as 20% or 12.50% (at most two decimal places)";
    let number = value.trim().strip_suffix('%').ok_or(INVALID)?.trim();
    let (whole, fraction) = number.split_once('.').unwrap_or((number, ""));
    if whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || fraction.len() > 2
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        || (number.contains('.') && fraction.is_empty())
    {
        return Err(INVALID);
    }
    let whole = whole.parse::<u32>().map_err(|_| INVALID)?;
    let fraction = match fraction.len() {
        0 => 0,
        1 => fraction.parse::<u32>().map_err(|_| INVALID)? * 10,
        _ => fraction.parse::<u32>().map_err(|_| INVALID)?,
    };
    whole
        .checked_mul(100)
        .and_then(|value| value.checked_add(fraction))
        .ok_or(INVALID)
}
