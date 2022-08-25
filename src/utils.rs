use near_sdk::Balance;

pub const MIN_PLAYTIME: u128 = 5 * 60;
pub const MAX_PLAYTIME: u128 = 60 * 60;
pub const DEFAULT_PLAYTIME: u128 = 20 * 60;

pub const MIN_BID: Balance = 2 * 10u128.pow(24);
pub const MAX_BID: Balance = 100 * 10u128.pow(24);

pub const FEE: Balance = 1 * 10u128.pow(23);

pub const ROKETO_ACC: &str = "streaming-r-v2.dcversus.testnet";
pub const WRAP_ACC: &str = "wrap.testnet";
// pub const ROKETO_ACC: &str = "streaming.r-v2.near";
// pub const WRAP_ACC: &str = "wrap.near";
