#![allow(dead_code)]

use serde::{Deserialize, Serialize};

macro_rules! newtype {
    ($name:ident, $inner:ty) => {
        #[derive(
            Copy,
            Clone,
            Debug,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            Serialize,
            Deserialize,
            Default,
        )]
        pub struct $name(pub $inner);
    };
}

newtype!(BasisBp, i32);
newtype!(Pp, u16);
newtype!(EconomyDay, u32);
newtype!(HubId, u16);
newtype!(CommodityId, u16);
newtype!(RouteId, u16);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Weather {
    #[default]
    Clear,
    Rains,
    Fog,
    Windy,
}
