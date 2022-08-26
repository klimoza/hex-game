use near_sdk::{Balance, Gas, ONE_NEAR};

pub const MIN_PLAYTIME: u32 = 5 * 60;
pub const MAX_PLAYTIME: u32 = 60 * 60;
pub const DEFAULT_PLAYTIME: u32 = 20 * 60;

pub const MIN_BID: Balance = 2 * ONE_NEAR;
pub const MAX_BID: Balance = 100 * ONE_NEAR;

pub const MIN_MAKE_BID_GAS: Gas = Gas(300 * ONE_TERA);
pub const MIN_MAKE_MOVE_GAS: Gas = Gas(300 * ONE_TERA);

pub const ONE_TERA: u64 = Gas::ONE_TERA.0;
pub const FEE: Balance = 2 * 10u128.pow(23);

pub const ROKETO_ACC: &str = "streaming-r-v2.dcversus.testnet";
pub const WRAP_ACC: &str = "wrap.testnet";
// pub const ROKETO_ACC: &str = "streaming.r-v2.near";
// pub const WRAP_ACC: &str = "wrap.near";
