#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(
    Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub struct MoneyCents(pub i64);

impl MoneyCents {
    pub const ZERO: Self = Self(0);

    pub fn saturating_add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }

    pub fn saturating_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }

    pub(crate) fn from_i128_clamped(value: i128) -> Self {
        const MIN: i128 = i64::MIN as i128;
        const MAX: i128 = i64::MAX as i128;
        if value < MIN {
            Self(i64::MIN)
        } else if value > MAX {
            Self(i64::MAX)
        } else {
            Self(value as i64)
        }
    }

    pub fn as_i64(self) -> i64 {
        self.0
    }
}

impl From<i64> for MoneyCents {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

impl From<MoneyCents> for i64 {
    fn from(value: MoneyCents) -> Self {
        value.0
    }
}
