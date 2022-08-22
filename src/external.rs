use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    ext_contract,
    json_types::{Base58CryptoHash, U128},
    serde::{Deserialize, Serialize},
    AccountId, Balance, CryptoHash, Promise, Timestamp,
};
use std::collections::HashMap;

use crate::*;

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

pub mod b58_dec_format {
    use near_sdk::json_types::Base58CryptoHash;
    use near_sdk::serde::{Deserialize, Deserializer, Serializer};
    use near_sdk::CryptoHash;

    pub fn serialize<S>(val: &CryptoHash, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // impossible to do without intermediate serialization
        serializer.serialize_str(&String::from(&Base58CryptoHash::from(*val)))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<CryptoHash, D::Error>
    where
        D: Deserializer<'de>,
    {
        // same as above
        Ok(Base58CryptoHash::deserialize(deserializer)?.into())
    }
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, PartialEq, Debug)]
#[serde(crate = "near_sdk::serde")]
pub enum StreamFinishReason {
    StoppedByOwner,
    StoppedByReceiver,
    FinishedNaturally,
    FinishedBecauseCannotBeExtended,
    FinishedWhileTransferred,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, PartialEq, Debug)]
#[serde(crate = "near_sdk::serde")]
pub enum StreamStatus {
    Initialized,
    Active,
    Paused,
    Finished { reason: StreamFinishReason },
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug))]
#[serde(crate = "near_sdk::serde")]
pub struct Stream {
    #[serde(with = "b58_dec_format")]
    pub id: CryptoHash,
    pub description: Option<String>,
    pub creator_id: AccountId,
    pub owner_id: AccountId,
    pub receiver_id: AccountId,
    pub token_account_id: AccountId,

    pub timestamp_created: Timestamp,
    pub last_action: Timestamp,

    #[serde(with = "u128_dec_format")]
    pub balance: Balance,
    #[serde(with = "u128_dec_format")]
    pub tokens_per_sec: Balance,

    pub status: StreamStatus,
    #[serde(with = "u128_dec_format")]
    pub tokens_total_withdrawn: Balance,

    // Cliff is a moment of time which divides the stream into two parts:
    // - before the cliff - withdraw is disabled;
    // - after the cliff - stream becomes a regular one.
    //
    // Streams with cliff must be started immediately.
    // Additionally, pausing streams with cliff is disabled
    // because it's hard to predict the proper behavior of the stream.
    //
    // The only way to withdraw tokens from stream with active cliff
    // is to stop it completely. In this case we take commission
    // that is proportional to time passed for total stream time.
    //
    // The reason of having cliffs is to reproduce vesting contracts.
    pub cliff: Option<Timestamp>,

    // Stream non-expiration is a hard concept to understand.
    //
    // The idea is based on observation that no stream can be stopped
    // automatically with no action provided. So, if receiver haven't
    // withdraw his tokens from fully expired stream yet,
    // the stream is considered Active.
    //
    // This basically means, the owner can deposit tokens onto the stream
    // even it's already expired, as long as receiver haven't tried to withdraw
    // the tokens that leads to stream finishing. In other terms,
    // it's possible to have a credit that may be covered later.
    //
    // Such behavior called non-expirable streams and disabled by default.
    // Expirable streams will be terminated even on stream depositing.
    pub is_expirable: bool,

    // Locked streams are ones that are unable to pause, stop and change receiver.
    // Locked streams are still may be terminated or deposited until started.
    //
    // For locked streams we take commission when the stream is started,
    // to allow us to own and handle commission tokens without waiting
    // as the final result of locked stream cannot be changed.
    pub is_locked: bool,

    #[borsh_skip]
    #[serde(with = "u128_dec_format")]
    pub available_to_withdraw_by_formula: Balance,
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
    fn get_stream(self, stream_id: Base58CryptoHash) -> Stream;
    fn pause_stream(&mut self, stream_id: Base58CryptoHash) -> Vec<Promise>;
}
