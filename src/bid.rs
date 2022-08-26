use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    json_types::Base58CryptoHash,
    Promise, PromiseResult,
};

use crate::{
    external::AccountView,
    game::Player,
    roketo::{get_two_streams, roketo_create_stream, roketo_get_account, stop_stream},
    utils::{FEE, MIN_MAKE_BID_GAS},
    *,
};

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Bid {
    pub bid: u128,
    pub did_first_player_bet: bool,
    pub did_second_player_bet: bool,
    pub stream_to_first_player: Base58CryptoHash,
    pub stream_to_second_player: Base58CryptoHash,
}

impl Bid {
    pub fn new(bid: u128) -> Self {
        Self {
            bid,
            did_first_player_bet: false,
            did_second_player_bet: false,
            stream_to_first_player: Base58CryptoHash::default(),
            stream_to_second_player: Base58CryptoHash::default(),
        }
    }

    pub fn stop_streams(&self) -> Promise {
        stop_stream(self.stream_to_first_player).then(stop_stream(self.stream_to_second_player))
    }
}

#[near_bindgen]
impl Contract {
    #[payable]
    pub fn make_bid(&mut self, game_id: GameIndex) -> Promise {
        let opt_bid = self.bids.get(&game_id);
        require!(
            opt_bid.is_some(),
            "There's no betting game with such index."
        );
        require!(
            env::prepaid_gas() >= MIN_MAKE_BID_GAS,
            "You should attach more gas"
        );

        let game = self.games.get(&game_id).unwrap().game;
        let bid = opt_bid.unwrap();
        let account_id = env::predecessor_account_id();

        require!(env::attached_deposit() >= 2 * bid.bid + FEE);

        if account_id == game.first_player && !bid.did_first_player_bet {
            roketo_create_stream(bid.bid, game.playtime.unwrap(), account_id)
                .then(roketo_get_account(env::current_account_id()))
                .then(Self::ext(env::current_account_id()).resolve_player_bid(
                    bid,
                    game_id,
                    Player::First,
                ))
        } else if account_id == game.second_player && !bid.did_second_player_bet {
            roketo_create_stream(bid.bid, game.playtime.unwrap(), account_id)
                .then(roketo_get_account(env::current_account_id()))
                .then(Self::ext(env::current_account_id()).resolve_player_bid(
                    bid,
                    game_id,
                    Player::Second,
                ))
        } else {
            require!(false, "Invalid bet");
            unreachable!();
        }
    }

    #[private]
    pub fn resolve_player_bid(&mut self, bid: Bid, game_id: GameIndex, player: Player) {
        require!(env::predecessor_account_id() == env::current_account_id());
        require!(env::promise_results_count() == 1, "ERR_TOO_MANY_RESULTS");
        let stream_id = match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(val) => {
                if let Ok(account) = near_sdk::serde_json::from_slice::<AccountView>(&val) {
                    account.last_created_stream.unwrap()
                } else {
                    env::panic_str("ERR_WRONG_VAL_RECEIVED")
                }
            }
            PromiseResult::Failed => env::panic_str("ERR_CALL_FAILED"),
        };
        let new_bid = match player {
            Player::First => Bid {
                did_first_player_bet: true,
                stream_to_first_player: stream_id,
                ..bid
            },
            Player::Second => Bid {
                did_second_player_bet: true,
                stream_to_second_player: stream_id,
                ..bid
            },
        };
        self.bids.insert(&game_id, &new_bid);
    }

    pub(crate) fn player_won(&self, bid: &Bid, game: &Game, player: Player) -> Promise {
        match player {
            Player::First => Promise::new(game.first_player.clone()).transfer(bid.bid),
            Player::Second => Promise::new(game.second_player.clone()).transfer(bid.bid),
        }
    }

    pub(crate) fn check_stream_bids(&mut self, game_id: GameIndex) -> Option<Promise> {
        let bid = self.bids.get(&game_id);
        if bid.is_none() {
            None
        } else {
            let unwrap_bid = bid.unwrap();
            Some(
                get_two_streams(
                    unwrap_bid.stream_to_first_player,
                    unwrap_bid.stream_to_second_player,
                )
                .then(Self::ext(env::current_account_id()).parse_two_promise_streams()),
            )
        }
    }
}
