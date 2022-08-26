use near_contract_standards::fungible_token::core::ext_ft_core;
use near_sdk::{
    json_types::{Base58CryptoHash, U128},
    Gas, Promise,
};

use crate::{
    external::{ext_roketo, ext_wrap},
    utils::{ONE_TERA, ROKETO_ACC, WRAP_ACC},
    *,
};

pub(crate) fn roketo_create_stream(
    bid: u128,
    game_playtime: u32,
    receiver_id: AccountId,
) -> Promise {
    let tokens_per_sec = (bid + game_playtime as u128 - 1) / game_playtime as u128;
    let fee = (bid + 9) / 10;
    let msg = format!("{{\"Create\":{{\"request\":{{\"balance\":\"{}\", \"owner_id\":\"{}\",\"receiver_id\":\"{}\",\"token_name\": \"wrap.testnet\", \"tokens_per_sec\":\"{}\"}}}}}}", bid, env::current_account_id(), receiver_id.to_string(), tokens_per_sec);
    ext_wrap::ext(WRAP_ACC.parse().unwrap())
        .with_attached_deposit(bid + fee)
        .near_deposit()
        .then(
            ext_ft_core::ext(WRAP_ACC.parse().unwrap())
                .with_attached_deposit(1)
                .with_static_gas(Gas(50 * ONE_TERA))
                .ft_transfer_call(
                    ROKETO_ACC.parse().unwrap(),
                    U128::from(bid + fee),
                    Some(String::from("Roketo transfer")),
                    msg,
                ),
        )
}

pub(crate) fn roketo_get_account(account_id: AccountId) -> Promise {
    ext_roketo::ext(ROKETO_ACC.parse().unwrap()).get_account(account_id, None)
}

pub(crate) fn roketo_get_stream(stream_id: Base58CryptoHash) -> Promise {
    ext_roketo::ext(ROKETO_ACC.parse().unwrap()).get_stream(stream_id)
}

pub(crate) fn get_two_streams(
    stream_id1: Base58CryptoHash,
    stream_id2: Base58CryptoHash,
) -> Promise {
    roketo_get_stream(stream_id1).and(roketo_get_stream(stream_id2))
}

pub(crate) fn pause_stream(stream_id: Base58CryptoHash) -> Promise {
    ext_roketo::ext(ROKETO_ACC.parse().unwrap())
        .with_attached_deposit(1)
        .with_static_gas(Gas(50 * ONE_TERA))
        .pause_stream(stream_id)
}

pub(crate) fn stop_stream(stream_id: Base58CryptoHash) -> Promise {
    ext_roketo::ext(ROKETO_ACC.parse().unwrap())
        .with_attached_deposit(1)
        .stop_stream(stream_id)
}

pub(crate) fn start_stream(stream_id: Base58CryptoHash) -> Promise {
    ext_roketo::ext(ROKETO_ACC.parse().unwrap())
        .with_attached_deposit(1)
        .start_stream(stream_id)
}
