use bid::Bid;
use cell::Cell;
use game::{Game, GameIndex};
use game_with_data::GameWithData;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    env, near_bindgen, require, AccountId, BorshStorageKey, PanicOnDefault, Promise, PromiseResult,
};
use roketo::start_stream;
use utils::BID;

use crate::external::{Stream, StreamFinishReason, StreamStatus};
use crate::roketo::{pause_stream, stop_stream};

#[derive(BorshSerialize, BorshStorageKey)]
pub enum StorageKey {
    Games,
    Field { game_id: GameIndex },
    Bid,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
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

    pub fn create_game(
        &mut self,
        first_player: AccountId,
        second_player: AccountId,
        field_size: Option<usize>,
        bid: Option<u128>,
    ) -> GameIndex {
        if bid.is_some() {
            require!(
                bid.unwrap() != 0 && bid.unwrap() <= BID,
                "Bid can't be too big and shouldn't be a zero."
            );
        }
        let index = self.next_game_id;
        let size = field_size.unwrap_or(11);
        self.games.insert(
            &index,
            &GameWithData::new(first_player, second_player, size),
        );

        if bid.is_some() {
            self.bids.insert(&index, &Bid::new(bid.unwrap()));
        }

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

    pub fn get_game_internal(&self, index: GameIndex) -> Game {
        let game = self.games.get(&index).map(|x| x.game);
        game.unwrap()
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
        // TODO: проверить что все ставки уже сделаны
        if let Some(promise) = self.check_stream_bids(index) {
            promise
                .then(Self::ext(env::current_account_id()).resolve_streams(index, move_type, cell))
        } else {
            Self::ext(env::current_account_id()).make_move_internal(index, move_type, cell)
        }
    }

    pub fn resolve_streams(
        &mut self,
        game_id: GameIndex,
        move_type: MoveType,
        cell: Option<Cell>,
    ) -> Promise {
        require!(env::predecessor_account_id() == env::current_account_id());
        require!(env::promise_results_count() == 1, "ERR_TOO_MANY_RESULTS");
        let (res, stream1, stream2) = match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(val) => {
                if let Ok(res) = near_sdk::serde_json::from_slice::<(u8, Stream, Stream)>(&val) {
                    res
                } else {
                    env::panic_str("ERR_WRONG_VAL_RECEIVED")
                }
            }
            PromiseResult::Failed => env::panic_str("ERR_CALL_FAILED"),
        };
        let bid = self.bids.get(&game_id).unwrap();
        let mut game_with_data = self.games.get(&game_id).unwrap();
        game_with_data.game.is_finished = true;
        self.games.insert(&game_id, &game_with_data);
        let game = game_with_data.game;
        match res {
            0 => Self::ext(env::current_account_id()).make_move_internal(game_id, move_type, cell),
            1 => {
                let bal = stream2.balance;
                stop_stream(stream2.id.into())
                    .then(Promise::new(game.first_player.clone()).transfer(bal + bid.bid))
            }
            2 => {
                let bal = stream1.balance;
                stop_stream(stream1.id.into())
                    .then(Promise::new(game.second_player.clone()).transfer(bal + bid.bid))
            }
            3 => Promise::new(game.first_player.clone())
                .transfer(bid.bid)
                .then(Promise::new(game.second_player.clone()).transfer(bid.bid)),
            _ => unreachable!(),
        }
    }

    pub fn make_move_internal(
        &mut self,
        index: GameIndex,
        move_type: MoveType,
        cell: Option<Cell>,
    ) -> Promise {
        let mut game_with_data = self.games.get(&index).expect("Game doesn't exist.");
        if game_with_data.game.is_finished {
            return Self::ext(env::current_account_id()).get_game_internal(index);
        }
        let old_board = game_with_data.game.board.clone();

        game_with_data.make_move(move_type, cell);

        env::log_str("Old board:");
        old_board.debug_logs();

        env::log_str("New board:");
        game_with_data.game.board.debug_logs();

        self.games.insert(&index, &game_with_data);

        if game_with_data.game.is_finished {
            if game_with_data.game.turn % 2 == 1 {
                env::log_str("First player wins!");
            } else {
                env::log_str("Second player wins!");
            }
            if let Some(bid) = self.bids.get(&index) {
                bid.stop_streams()
                    .then(self.player_won(
                        &bid,
                        &game_with_data.game,
                        game_with_data.game.turn as u8 % 2,
                    ))
                    .then(Self::ext(env::current_account_id()).get_game_internal(index))
            } else {
                Self::ext(env::current_account_id()).get_game_internal(index)
            }
        } else {
            if let Some(bid) = self.bids.get(&index) {
                if game_with_data.game.turn % 2 == 1 {
                    start_stream(bid.stream_to_second_player)
                        .then(Self::ext(env::current_account_id()).get_game_internal(index))
                } else {
                    start_stream(bid.stream_to_first_player)
                        .then(Self::ext(env::current_account_id()).get_game_internal(index))
                }
            } else {
                Self::ext(env::current_account_id()).get_game_internal(index)
            }
        }
    }

    pub fn parse_two_promise_streams(&mut self) -> Promise {
        require!(env::predecessor_account_id() == env::current_account_id());
        require!(env::promise_results_count() == 2, "ERR_TOO_MANY_RESULTS");
        let stream1 = match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(val) => {
                if let Ok(stream) = near_sdk::serde_json::from_slice::<Stream>(&val) {
                    stream
                } else {
                    env::panic_str("ERR_WRONG_VAL_RECEIVED")
                }
            }
            PromiseResult::Failed => env::panic_str("ERR_CALL_FAILED"),
        };
        let stream2 = match env::promise_result(1) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(val) => {
                if let Ok(stream) = near_sdk::serde_json::from_slice::<Stream>(&val) {
                    stream
                } else {
                    env::panic_str("ERR_WRONG_VAL_RECEIVED")
                }
            }
            PromiseResult::Failed => env::panic_str("ERR_CALL_FAILED"),
        };
        if stream1.status == StreamStatus::Active && stream2.status == StreamStatus::Active {
            pause_stream(stream1.id.into())
                .and(pause_stream(stream2.id.into()))
                .then(Self::ext(env::current_account_id()).parse_two_streams(stream1, stream2))
        } else if stream1.status == StreamStatus::Active {
            pause_stream(stream1.id.into())
                .then(Self::ext(env::current_account_id()).parse_two_streams(stream1, stream2))
        } else if stream2.status == StreamStatus::Active {
            pause_stream(stream2.id.into())
                .then(Self::ext(env::current_account_id()).parse_two_streams(stream1, stream2))
        } else {
            Self::ext(env::current_account_id()).parse_two_streams(stream1, stream2)
        }
    }

    pub fn parse_two_streams(&mut self, stream1: Stream, stream2: Stream) -> (u8, Stream, Stream) {
        require!(env::predecessor_account_id() == env::current_account_id());
        match (stream1.status.clone(), stream2.status.clone()) {
            (StreamStatus::Active, _) => unreachable!(),
            (_, StreamStatus::Active) => unreachable!(),
            (
                _,
                StreamStatus::Finished {
                    reason: StreamFinishReason::StoppedByOwner,
                },
            ) => unreachable!(),
            (
                StreamStatus::Finished {
                    reason: StreamFinishReason::StoppedByOwner,
                },
                _,
            ) => unreachable!(),
            (
                _,
                StreamStatus::Finished {
                    reason: StreamFinishReason::FinishedWhileTransferred,
                },
            ) => unreachable!(),
            (
                StreamStatus::Finished {
                    reason: StreamFinishReason::FinishedWhileTransferred,
                },
                _,
            ) => unreachable!(),
            (
                _,
                StreamStatus::Finished {
                    reason: StreamFinishReason::FinishedBecauseCannotBeExtended,
                },
            ) => unreachable!(),
            (
                StreamStatus::Finished {
                    reason: StreamFinishReason::FinishedBecauseCannotBeExtended,
                },
                _,
            ) => unreachable!(),
            (
                _,
                StreamStatus::Finished {
                    reason: StreamFinishReason::StoppedByReceiver,
                },
            ) => (0, stream1, stream2),
            (
                StreamStatus::Finished {
                    reason: StreamFinishReason::StoppedByReceiver,
                },
                _,
            ) => (0, stream1, stream2),
            (
                StreamStatus::Finished {
                    reason: StreamFinishReason::FinishedNaturally,
                },
                StreamStatus::Finished {
                    reason: StreamFinishReason::FinishedNaturally,
                },
            ) => (3, stream1, stream2),
            (
                StreamStatus::Finished {
                    reason: StreamFinishReason::FinishedNaturally,
                },
                _,
            ) => (1, stream1, stream2),
            (
                _,
                StreamStatus::Finished {
                    reason: StreamFinishReason::FinishedNaturally,
                },
            ) => (2, stream1, stream2),
            _ => (0, stream1, stream2),
        }
    }
}

pub mod bid;
pub mod board;
pub mod cell;
pub mod external;
pub mod game;
pub mod game_with_data;
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
        contract.create_game(accounts(1), accounts(2), Some(3), None);
        contract.create_game(accounts(4), accounts(3), Some(4), None);
        let id = contract.create_game(accounts(0), accounts(1), None, None);
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
        let id = contract.create_game(accounts(0), accounts(1), Some(5), None);

        testing_env!(get_context(accounts(0)));
        let mut test_game = GameWithData::new(accounts(0), accounts(1), 5);
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
