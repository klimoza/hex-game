use bid::Bid;
use cell::Cell;
use game::{Game, GameIndex};
use game_with_data::GameWithData;
use near_contract_standards::non_fungible_token::refund_deposit;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    env, near_bindgen, require, AccountId, BorshStorageKey, PanicOnDefault, Promise, PromiseResult,
};
use roketo::start_stream;
use utils::{DEFAULT_PLAYTIME, MIN_BID};

use crate::external::{Stream, StreamFinishReason, StreamStatus};
use crate::roketo::{pause_stream, stop_stream};
use crate::utils::{MAX_BID, MAX_PLAYTIME, MIN_PLAYTIME};

#[derive(BorshSerialize, BorshStorageKey)]
pub enum StorageKey {
    Games,
    Field { game_id: GameIndex },
    Bid,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub enum MoveType {
    PLACE,
    SWAP,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    pub games: LookupMap<GameIndex, GameWithData>,
    pub bids: LookupMap<GameIndex, Bid>,
    pub next_game_id: u64,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        Self {
            games: LookupMap::new(StorageKey::Games),
            bids: LookupMap::new(StorageKey::Bid),
            next_game_id: 0,
        }
    }

    #[payable]
    pub fn create_game(
        &mut self,
        first_player: AccountId,
        second_player: AccountId,
        field_size: Option<usize>,
        bid: Option<U128>,
        playtime: Option<u32>,
    ) -> GameIndex {
        if playtime.is_some() {
            require!(
                playtime.unwrap() >= MIN_PLAYTIME && playtime.unwrap() <= MAX_PLAYTIME,
                "Game playtime can't be too small or too big."
            );
            require!(
                bid.is_some(),
                "You can't make game with time control without betting."
            )
        }
        let game_bid = bid.map(|x| u128::from(x));
        if game_bid.is_some() {
            require!(
                game_bid.unwrap() >= MIN_BID && game_bid.unwrap() <= MAX_BID,
                "Bid can't be too small or too big."
            );
        }
        let initial_storage_usage = env::storage_usage();

        let index = self.next_game_id;
        let size = field_size.unwrap_or(11);
        let game_playtime = if game_bid.is_some() {
            if playtime.is_some() {
                playtime
            } else {
                Some(DEFAULT_PLAYTIME)
            }
        } else {
            None
        };

        self.games.insert(
            &index,
            &GameWithData::new(first_player, second_player, size, game_playtime),
        );

        if game_bid.is_some() {
            self.bids.insert(&index, &Bid::new(game_bid.unwrap()));
        }

        let required_storage_in_bytes = env::storage_usage() - initial_storage_usage;
        refund_deposit(required_storage_in_bytes);

        env::log_str("Created board:");
        self.games.get(&index).unwrap().game.board.debug_logs();
        self.next_game_id += 1;
        index
    }

    pub fn get_game(&self, index: GameIndex) -> Option<Game> {
        let game = self.games.get(&index).map(|x| x.game);
        if game.is_some() {
            env::log_str("Game board:");
            game.clone().unwrap().board.debug_logs();
        }
        game
    }

    pub fn make_move(
        &mut self,
        index: GameIndex,
        move_type: MoveType,
        cell: Option<Cell>,
    ) -> Promise {
        let game_with_data = self.games.get(&index).expect("Game doesn't exist.");
        require!(
            !game_with_data.game.is_finished,
            "Game is already finished!"
        );
        let bid = self.bids.get(&index);
        if let Some(bid) = bid {
            require!(
                bid.did_first_player_bet && bid.did_second_player_bet,
                "Players should deposit their bets before game start."
            );
        }
        let game = game_with_data.game;
        match (move_type.clone(), cell.clone()) {
            (MoveType::PLACE, Some(cell)) => {
                if game.turn % 2 == 0 {
                    require!(
                        env::predecessor_account_id() == game.first_player,
                        "It's not your turn"
                    );
                } else {
                    require!(
                        env::predecessor_account_id() == game.second_player,
                        "It's not your turn"
                    );
                }
                require!(game.board.get_cell(&cell) == 0, "Cell is already filled.");
            }
            (MoveType::SWAP, _) => {
                require!(
                    env::predecessor_account_id() == game.second_player,
                    "Incorrect predecessor account"
                );
                require!(
                    game.turn == 1,
                    "You can apply swap rule only on the second turn"
                );
            }
            _ => env::panic_str("Incorrect move args"),
        };

        // require!(
        //     env::prepaid_gas() >= MIN_MAKE_MOVE_GAS,
        //     "You should attach more gas."
        // );

        if let Some(promise) = self.check_stream_bids(index) {
            promise
                .then(Self::ext(env::current_account_id()).resolve_streams(index, move_type, cell))
        } else {
            Self::ext(env::current_account_id()).make_move_internal(index, move_type, cell)
        }
    }
}

pub mod bid;
pub mod board;
pub mod cell;
pub mod external;
pub mod game;
pub mod game_with_data;
pub mod internal;
pub mod roketo;
pub mod utils;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod contract_tests {
    use core::fmt::Debug;
    use near_sdk::{
        test_utils::{accounts, VMContextBuilder},
        testing_env, AccountId,
    };

    use crate::{
        board::Board, cell::Cell, game::Game, game_with_data::GameWithData, Contract, MoveType,
    };

    fn get_context(account: AccountId) -> near_sdk::VMContext {
        VMContextBuilder::new()
            .predecessor_account_id(account)
            .build()
    }

    impl PartialEq for Game {
        fn eq(&self, other: &Self) -> bool {
            self.first_player == other.first_player
                && self.second_player == other.second_player
                && self.turn == other.turn
                && self.board == other.board
                && self.current_block_height == other.current_block_height
                && self.prev_block_height == other.prev_block_height
                && self.is_finished == other.is_finished
        }
    }

    impl PartialEq for GameWithData {
        fn eq(&self, other: &Self) -> bool {
            self.game == other.game && self.data == other.data
        }
    }

    impl Debug for Game {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Game")
                .field("first_player", &self.first_player)
                .field("second_player", &self.second_player)
                .field("turn", &self.turn)
                .field("board", &self.board)
                .field("current_block_height", &self.current_block_height)
                .field("prev_block_height", &self.prev_block_height)
                .field("is_finished", &self.is_finished)
                .finish()
        }
    }

    impl Debug for GameWithData {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("GameWithData")
                .field("game", &self.game)
                .field("data", &self.data)
                .finish()
        }
    }

    #[test]
    fn test_create_get() {
        let mut contract = Contract::new();
        contract.create_game(accounts(1), accounts(2), Some(3), None, None);
        contract.create_game(accounts(4), accounts(3), Some(4), None, None);
        let id = contract.create_game(accounts(0), accounts(1), None, None, None);
        assert_eq!(id, 2);
        let game = contract.get_game(id);

        assert!(contract.get_game(id + 1).is_none());
        assert!(game.is_some());
        assert_eq!(game.clone().unwrap().first_player, accounts(0));
        assert_eq!(game.clone().unwrap().second_player, accounts(1));
        assert_eq!(game.unwrap().board, Board::new(11));
    }

    #[test]
    fn test_make_move() {
        let mut contract = Contract::new();
        let id = contract.create_game(accounts(0), accounts(1), Some(5), None, None);

        testing_env!(get_context(accounts(0)));
        let mut test_game = GameWithData::new(accounts(0), accounts(1), 5, None);
        assert_eq!(test_game, contract.games.get(&id).unwrap());

        // let game = contract.make_move(id, MoveType::PLACE, Some(Cell::new(4, 0)));
        test_game.make_move(MoveType::PLACE, Some(Cell::new(4, 0)));
        // assert_eq!(test_game.game, game);
        assert_eq!(test_game, contract.games.get(&id).unwrap());

        testing_env!(get_context(accounts(1)));
        // let game = contract.make_move(id, MoveType::SWAP, Some(Cell::new(4, 0)));
        test_game.make_move(MoveType::SWAP, Some(Cell::new(4, 0)));
        // assert_eq!(test_game.game, game);
        assert_eq!(test_game, contract.games.get(&id).unwrap());
    }
}
