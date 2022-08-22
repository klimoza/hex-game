use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    json_types::Base58CryptoHash,
    Promise,
};

use crate::{
    stream::{create_stream, get_roketo_account},
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
            create_stream(bid.bid, GAME_PLAYTIME, game.first_player)
                .then(get_roketo_account(account_id))
                .then(Self::ext(env::current_account_id()).resolve_first_player(bid, game_id))
        } else if account_id == game.second_player && !bid.did_second_player_bet {
            create_stream(bid.bid, GAME_PLAYTIME, game.second_player)
                .then(get_roketo_account(account_id))
                .then(Self::ext(env::predecessor_account_id()).resolve_second_player(bid, game_id))
        } else {
            require!(false, "Incorrect bet");
            unreachable!();
        }
        // TODO: сделать проверку на количество денег
    }
}
