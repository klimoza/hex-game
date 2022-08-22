use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    json_types::Base58CryptoHash,
    Promise, PromiseResult,
};

use crate::{
    external::AccountView,
    roketo::{get_two_streams, roketo_create_stream, roketo_get_account},
    utils::GAME_PLAYTIME,
    *,
};

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Bid {
    pub bid: u128,
    pub did_first_player_bet: bool,
    pub did_second_player_bet: bool,
    pub stream_from_first_player: Base58CryptoHash,
    pub stream_from_second_player: Base58CryptoHash,
}

impl Bid {
    pub fn new(bid: u128) -> Self {
        Self {
            bid,
            did_first_player_bet: false,
            did_second_player_bet: false,
            stream_from_first_player: Base58CryptoHash::default(),
            stream_from_second_player: Base58CryptoHash::default(),
        }
    }
}

#[near_bindgen]
impl Contract {
    pub(crate) fn player_won(&self, bid: &Bid, game: &Game, player: u8) -> Promise {
        if player == 1 {
            Promise::new(game.first_player.clone()).transfer(bid.bid)
        } else {
            Promise::new(game.second_player.clone()).transfer(bid.bid)
        }
    }

    #[payable]
    pub fn make_bid(&mut self, game_id: GameIndex) -> Promise {
        let opt_bid = self.bids.get(&game_id);
        require!(
            opt_bid.is_some(),
            "There's no betting game with such index."
        );

        let game = self.games.get(&game_id).unwrap().game;
        let bid = opt_bid.unwrap();
        let account_id = env::predecessor_account_id();

        if account_id == game.first_player && !bid.did_first_player_bet {
            roketo_create_stream(bid.bid, GAME_PLAYTIME, game.first_player)
                .then(roketo_get_account(account_id))
                .then(Self::ext(env::current_account_id()).bid_resolve_first_player(bid, game_id))
        } else if account_id == game.second_player && !bid.did_second_player_bet {
            roketo_create_stream(bid.bid, GAME_PLAYTIME, game.second_player)
                .then(roketo_get_account(account_id))
                .then(
                    Self::ext(env::predecessor_account_id())
                        .bid_resolve_second_player(bid, game_id),
                )
        } else {
            require!(false, "Incorrect bet");
            unreachable!();
        }
        // TODO: сделать проверку на количество денег
    }

    pub fn bid_resolve_first_player(&mut self, bid: Bid, game_id: GameIndex) {
        require!(env::predecessor_account_id() == env::current_account_id());
        require!(env::promise_results_count() == 1, "ERR_TOO_MANY_RESULTS");
        let stream_id = match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(val) => {
                if let Ok(stream_id) = near_sdk::serde_json::from_slice::<Base58CryptoHash>(&val) {
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

    pub fn bid_resolve_second_player(&mut self, bid: Bid, game_id: GameIndex) {
        require!(env::predecessor_account_id() == env::current_account_id());
        require!(env::promise_results_count() == 1, "ERR_TOO_MANY_RESULTS");
        let stream_id = match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(val) => {
                if let Ok(account) = near_sdk::serde_json::from_slice::<AccountView>(&val) {
                    account.last_created_stream
                } else {
                    env::panic_str("ERR_WRONG_VAL_RECEIVED")
                }
            }
            PromiseResult::Failed => env::panic_str("ERR_CALL_FAILED"),
        };
        let new_bid = Bid {
            did_second_player_bet: true,
            stream_from_second_player: stream_id.unwrap(),
            ..bid
        };
        self.bids.insert(&game_id, &new_bid);
    }

    pub(crate) fn check_stream_bids(&mut self, game_id: GameIndex) -> Option<Promise> {
        require!(env::predecessor_account_id() == env::current_account_id());
        let bid = self.bids.get(&game_id);
        if bid.is_none() {
            None
        } else {
            let unwrap_bid = bid.unwrap();
            Some(
                get_two_streams(
                    unwrap_bid.stream_from_first_player,
                    unwrap_bid.stream_from_second_player,
                )
                .then(Self::ext(env::current_account_id()).parse_two_promise_streams(game_id)),
            )
        }
    }
}
