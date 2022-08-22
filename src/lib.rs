use bid::Bid;
use cell::Cell;
use game::{Game, GameIndex};
use game_with_data::GameWithData;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, near_bindgen, require, AccountId, BorshStorageKey, PanicOnDefault};
use utils::BID;

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

    pub fn make_move(&mut self, index: GameIndex, move_type: MoveType, cell: Option<Cell>) -> Game {
        let mut game_with_data = self.games.get(&index).expect("Game doesn't exist.");
        require!(
            !game_with_data.game.is_finished,
            "Game is already finished!"
        );
        // TODO: проверить что все ставки уже сделаны
        let old_board = game_with_data.game.board.clone();
        game_with_data.make_move(move_type, cell);

        env::log_str("Old board:");
        old_board.debug_logs();

        env::log_str("New board:");
        game_with_data.game.board.debug_logs();

        if game_with_data.game.is_finished {
            if game_with_data.game.turn % 2 == 1 {
                env::log_str("First player wins!");
            } else {
                env::log_str("Second player wins!");
            }
        }

        self.games.insert(&index, &game_with_data);
        return self.games.get(&index).unwrap().game;
    }
}

pub mod bid;
pub mod board;
pub mod cell;
pub mod external;
pub mod game;
pub mod game_with_data;
pub mod stream;
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

        let game = contract.make_move(id, MoveType::PLACE, Some(Cell::new(4, 0)));
        test_game.make_move(MoveType::PLACE, Some(Cell::new(4, 0)));
        assert_eq!(test_game.game, game);
        assert_eq!(test_game, contract.games.get(&id).unwrap());

        testing_env!(get_context(accounts(1)));
        let game = contract.make_move(id, MoveType::SWAP, Some(Cell::new(4, 0)));
        test_game.make_move(MoveType::SWAP, Some(Cell::new(4, 0)));
        assert_eq!(test_game.game, game);
        assert_eq!(test_game, contract.games.get(&id).unwrap());
    }
}
