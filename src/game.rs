use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, AccountId, BlockHeight};

use crate::board::Board;
use crate::cell::Cell;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub enum Player {
    First,
    Second,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct Game {
    pub first_player: AccountId,
    pub second_player: AccountId,
    pub turn: usize,
    pub board: Board,
    pub current_block_height: BlockHeight,
    pub prev_block_height: BlockHeight,
    pub is_started: bool,
    pub is_finished: bool,
    pub playtime: Option<u32>,
    pub winner: Option<Player>,
}

pub type GameIndex = u64;

impl Game {
    pub fn new(
        first_player: AccountId,
        second_player: AccountId,
        field_size: usize,
        playtime: Option<u32>,
    ) -> Self {
        Self {
            first_player,
            second_player,
            turn: 0,
            board: Board::new(field_size),
            current_block_height: env::block_height(),
            prev_block_height: 0,
            is_started: false,
            is_finished: false,
            playtime,
            winner: None,
        }
    }

    pub fn place_counter(&mut self, cell: &Cell, player: u8) {
        self.board.set_cell(cell, player);
        self.turn += 1;
        if env::block_height() != self.current_block_height {
            self.prev_block_height = self.current_block_height;
            self.current_block_height = env::block_height();
        }
    }

    pub fn swap_rule(&mut self) -> Cell {
        let non_zero_byte = self
            .board
            .field
            .0
            .iter()
            .enumerate()
            .find(|(_, &x)| x != 0)
            .unwrap();
        let mut bit_number = 8 * non_zero_byte.0;
        if non_zero_byte.1 & 4 == 4 {
            bit_number += 2;
        } else if non_zero_byte.1 & 16 == 16 {
            bit_number += 4;
        } else if non_zero_byte.1 & 64 == 64 {
            bit_number += 6;
        }
        let cell = self.board.get_coords(bit_number);

        self.board.set_cell(&cell, 0);
        self.board.set_cell(&cell.symm(), 2);
        self.turn += 1;
        if env::block_height() != self.current_block_height {
            self.prev_block_height = self.current_block_height;
            self.current_block_height = env::block_height();
        }

        cell
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod game_tests {
    use near_sdk::{
        test_utils::{accounts, VMContextBuilder},
        testing_env,
    };

    use crate::cell::Cell;

    use super::Game;

    fn get_context() -> VMContextBuilder {
        VMContextBuilder::new()
    }

    #[test]
    #[should_panic]
    fn test_place_counter_wrong_player_1() {
        let mut game = Game::new(accounts(0), accounts(1), 11, None);
        game.place_counter(&Cell::new(1, 1), 2);
    }

    #[test]
    #[should_panic]
    fn test_place_counter_wrong_player_2() {
        let mut game = Game::new(accounts(0), accounts(1), 11, None);
        game.place_counter(&Cell::new(1, 1), 1);
        game.place_counter(&Cell::new(2, 1), 1);
    }

    #[test]
    #[should_panic]
    fn test_place_counter_cell_is_already_filled() {
        let mut game = Game::new(accounts(0), accounts(1), 11, None);
        game.place_counter(&Cell::new(1, 1), 1);
        game.place_counter(&Cell::new(1, 1), 2);
    }

    #[test]
    fn test_place_counter() {
        testing_env!(get_context().block_index(0).build());

        let mut game = Game::new(accounts(0), accounts(1), 11, None);
        game.place_counter(&Cell::new(1, 1), 1);
        game.place_counter(&Cell::new(1, 2), 2);
        game.place_counter(&Cell::new(10, 7), 1);
        assert_eq!(game.current_block_height, 0);
        assert_eq!(game.prev_block_height, 0);

        testing_env!(get_context().block_index(100).build());
        game.place_counter(&Cell::new(5, 9), 2);
        game.place_counter(&Cell::new(3, 7), 1);
        assert_eq!(game.current_block_height, 100);
        assert_eq!(game.prev_block_height, 0);

        assert_eq!(game.first_player, accounts(0));
        assert_eq!(game.second_player, accounts(1));
        assert_eq!(game.turn, 5);
        for i in 0..11 {
            for j in 0..11 {
                let cell = Cell::new(i, j);
                if cell == Cell::new(1, 1) || cell == Cell::new(10, 7) || cell == Cell::new(3, 7) {
                    assert_eq!(game.board.get_cell(&cell), 1);
                } else if cell == Cell::new(1, 2) || cell == Cell::new(5, 9) {
                    assert_eq!(game.board.get_cell(&cell), 2);
                } else {
                    assert_eq!(game.board.get_cell(&cell), 0);
                }
            }
        }
    }

    #[test]
    #[should_panic]
    fn test_swap_rule_too_early() {
        let mut game = Game::new(accounts(0), accounts(1), 11, None);
        game.swap_rule();
    }

    #[test]
    #[should_panic]
    fn test_swap_rule_too_late() {
        let mut game = Game::new(accounts(0), accounts(1), 11, None);
        game.place_counter(&Cell::new(2, 5), 1);
        game.place_counter(&Cell::new(10, 7), 2);
        game.swap_rule();
    }

    #[test]
    fn test_swap_rule() {
        let mut game = Game::new(accounts(0), accounts(1), 11, None);
        game.place_counter(&Cell::new(10, 7), 1);

        let c = game.swap_rule();
        assert_eq!(Cell::new(10, 7), c);
        assert_eq!(0, game.board.get_cell(&Cell::new(10, 7)));
        assert_eq!(2, game.board.get_cell(&Cell::new(7, 10)));

        game.place_counter(&Cell::new(1, 1), 1);
        game.place_counter(&Cell::new(5, 9), 2);
        game.place_counter(&Cell::new(3, 7), 1);

        assert_eq!(game.turn, 5);
        for i in 0..11 {
            for j in 0..11 {
                let cell = Cell::new(i, j);
                if cell == Cell::new(1, 1) || cell == Cell::new(3, 7) {
                    assert_eq!(game.board.get_cell(&cell), 1);
                } else if cell == Cell::new(7, 10) || cell == Cell::new(5, 9) {
                    assert_eq!(game.board.get_cell(&cell), 2);
                } else {
                    assert_eq!(game.board.get_cell(&cell), 0);
                }
            }
        }
    }
}
