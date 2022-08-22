use std::collections::HashMap;

use near_sdk::{
    ext_contract,
    json_types::{Base58CryptoHash, U128},
    serde::{Deserialize, Serialize},
    AccountId, Balance,
};

pub mod u128_dec_format {
    use near_sdk::serde::de;
    use near_sdk::serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(num: &u128, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&num.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u128, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(de::Error::custom)
    }
}

#[derive(Deserialize, Serialize)]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug))]
#[serde(crate = "near_sdk::serde")]
pub struct AccountView {
    pub active_incoming_streams: u32,
    pub active_outgoing_streams: u32,
    pub inactive_incoming_streams: u32,
    pub inactive_outgoing_streams: u32,

    pub total_incoming: HashMap<AccountId, U128>,
    pub total_outgoing: HashMap<AccountId, U128>,
    pub total_received: HashMap<AccountId, U128>,

    #[serde(with = "u128_dec_format")]
    pub deposit: Balance,

    #[serde(with = "u128_dec_format")]
    pub stake: Balance,

    pub last_created_stream: Option<Base58CryptoHash>,
    pub is_cron_allowed: bool,
}

#[ext_contract(ext_roketo)]
pub trait Roketo {
    fn get_account(self, account_id: AccountId, only_if_exist: Option<bool>) -> AccountView;
}
