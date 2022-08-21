use near_contract_standards::fungible_token::core::ext_ft_core;
use near_sdk::{json_types::U128, CryptoHash, Promise, PromiseResult};

use crate::*;

pub(crate) fn create_testnet_stream(
    bid: u128,
    game_playtime: u128,
    receiver_id: AccountId,
) -> Promise {
    let tokens_per_sec = (bid + game_playtime - 1) / game_playtime;
    let msg = format!("{{\"Create\":{{\"request\":{{\"owner_id\":\"{}\",\"receiver_id\":\"{}\",\"tokens_per_sec\":\"{}\"}}}}}}", env::current_account_id(), receiver_id.to_string(), tokens_per_sec);
    ext_ft_core::ext("wrap.testnet".parse().unwrap()).ft_transfer_call(
        "streaming-r-v2.dcversus.testnet".parse().unwrap(),
        U128::from(bid),
        Some(String::from("Roketo transfer")),
        msg,
    )
}

pub(crate) fn create_mainnet_stream(
    bid: u128,
    game_playtime: u128,
    receiver_id: AccountId,
) -> Promise {
    let tokens_per_sec = (bid + game_playtime - 1) / game_playtime;
    let msg = format!("{{\"Create\":{{\"request\":{{\"owner_id\":\"{}\",\"receiver_id\":\"{}\",\"tokens_per_sec\":\"{}\"}}}}}}", env::current_account_id(), receiver_id.to_string(), tokens_per_sec);
    ext_ft_core::ext("wrap.near".parse().unwrap()).ft_transfer_call(
        "streaming.r-v2.near".parse().unwrap(),
        U128::from(bid),
        Some(String::from("Roketo transfer")),
        msg,
    )
}

#[near_bindgen]
impl Contract {
    pub fn resolve_first_player(&mut self, bid: Bid, game_id: GameIndex) {
        require!(env::predecessor_account_id() == env::current_account_id());
        require!(env::promise_results_count() == 1, "ERR_TOO_MANY_RESULTS");
        let stream_id = match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(val) => {
                if let Ok(stream_id) = near_sdk::serde_json::from_slice::<CryptoHash>(&val) {
                    stream_id
                } else {
                    env::panic_str("ERR_WRONG_VAL_RECEIVED")
                }
            }
            PromiseResult::Failed => env::panic_str("ERR_CALL_FAILED"),
        };
        let new_bid = Bid {
            did_first_player_bet: true,
            stream_from_first_player: stream_id,
            ..bid
        };
        self.bids.insert(&game_id, &new_bid);
    }

    pub fn resolve_second_player(&mut self, bid: Bid, game_id: GameIndex) {
        require!(env::predecessor_account_id() == env::current_account_id());
        require!(env::promise_results_count() == 1, "ERR_TOO_MANY_RESULTS");
        let stream_id = match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(val) => {
                if let Ok(stream_id) = near_sdk::serde_json::from_slice::<CryptoHash>(&val) {
                    stream_id
                } else {
                    env::panic_str("ERR_WRONG_VAL_RECEIVED")
                }
            }
            PromiseResult::Failed => env::panic_str("ERR_CALL_FAILED"),
        };
        let new_bid = Bid {
            did_second_player_bet: true,
            stream_from_second_player: stream_id,
            ..bid
        };
        self.bids.insert(&game_id, &new_bid);
    }
}
