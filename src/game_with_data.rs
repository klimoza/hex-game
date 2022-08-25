use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, require, AccountId};
use std::collections::VecDeque;

use crate::board::Board;
use crate::cell::Cell;
use crate::game::Game;
use crate::*;

#[derive(BorshDeserialize, BorshSerialize)]
pub struct GameWithData {
    pub game: Game,
    pub data: Board,
}

impl GameWithData {
    pub fn new(
        first_player: AccountId,
        second_player: AccountId,
        field_size: usize,
        playtime: Option<u128>,
    ) -> Self {
        Self {
            game: Game::new(first_player, second_player, field_size, playtime),
            data: Board::new(field_size),
        }
    }

    pub fn make_move(&mut self, move_type: MoveType, cell: Option<Cell>) {
        match (move_type, cell) {
            (MoveType::PLACE, Some(cell)) => {
                if self.game.turn % 2 == 0 {
                    require!(
                        env::predecessor_account_id() == self.game.first_player,
                        "Incorrect predecessor account"
                    );
                    self.game.place_counter(&cell, 1);
                } else {
                    require!(
                        env::predecessor_account_id() == self.game.second_player,
                        "Incorrect predecessor account"
                    );
                    self.game.place_counter(&cell, 2);
                }
                self.process_cell(cell);
            }
            (MoveType::SWAP, _) => {
                require!(
                    env::predecessor_account_id() == self.game.second_player,
                    "Incorrect predecessor account"
                );
                let cell = self.game.swap_rule();
                self.data.set_cell(&cell, 0);
                self.process_cell(cell.symm());
            }
            _ => env::panic_str("Incorrect move args"),
        };
    }

    fn process_cell(&mut self, cell: Cell) {
        let color = self.game.board.get_cell(&cell);
        let (mut border1, mut border2) = if color == 1 {
            (cell.y == 0, cell.y + 1 == self.data.size)
        } else {
            (cell.x == 0, cell.x + 1 == self.data.size)
        };
        let neighbours = cell.get_neighbours(self.data.size);
        let good_neighbours = neighbours
            .iter()
            .filter(|c| self.game.board.get_cell(c) == color);
        border1 = border1 || good_neighbours.clone().any(|c| self.data.get_cell(c) == 1);
        border2 = border2 || good_neighbours.clone().any(|c| self.data.get_cell(c) == 2);
        if border1 && border2 {
            self.game.is_finished = true;
        } else if border1 {
            self.bfs(cell, color, 1);
        } else if border2 {
            self.bfs(cell, color, 2);
        }
    }

    fn bfs(&mut self, cell: Cell, color: u8, border: u8) {
        self.data.set_cell(&cell, border);
        let mut q: VecDeque<Cell> = VecDeque::new();
        q.push_back(cell);
        let field_size = self.data.size;
        while !q.is_empty() {
            let v = q.pop_front().unwrap();
            let neighbours = v.get_neighbours(field_size);
            let good_neighbours: Vec<Cell> = neighbours
                .into_iter()
                .filter(|c| self.game.board.get_cell(c) == color && self.data.get_cell(c) != border)
                .collect();
            if good_neighbours
                .clone()
                .into_iter()
                .any(|c| self.data.get_cell(&c) != 0)
            {
                self.game.is_finished = true;
                return;
            }
            for c in good_neighbours.into_iter() {
                self.data.set_cell(&c, border);
                q.push_back(Cell::new(c.x, c.y));
            }
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod game_with_board_tests {
    use std::fmt::Debug;

    use near_sdk::{
        test_utils::{accounts, VMContextBuilder},
        testing_env,
    };

    use super::*;

    impl PartialEq for Board {
        fn eq(&self, other: &Self) -> bool {
            self.size == other.size && self.field == other.field
        }
    }

    impl Debug for Board {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Board")
                .field("size", &self.size)
                .field("field", &self.field)
                .finish()
        }
    }

    fn get_context(account: AccountId) -> near_sdk::VMContext {
        VMContextBuilder::new()
            .predecessor_account_id(account)
            .build()
    }

    #[test]
    fn test_bfs() {
        let mut test_game = GameWithData::new(accounts(0), accounts(1), 5, None);
        test_game.game.board.set_cell(&Cell::new(0, 0), 1);
        test_game.game.board.set_cell(&Cell::new(0, 1), 1);
        test_game.game.board.set_cell(&Cell::new(0, 2), 1);

        test_game.game.board.set_cell(&Cell::new(4, 4), 1);
        test_game.game.board.set_cell(&Cell::new(3, 4), 1);

        test_game.game.board.set_cell(&Cell::new(0, 3), 1);
        test_game.game.board.set_cell(&Cell::new(1, 2), 1);

        test_game.game.board.set_cell(&Cell::new(4, 0), 2);

        test_game.game.board.set_cell(&Cell::new(2, 1), 1);

        test_game.game.board.set_cell(&Cell::new(3, 0), 2);

        let mut test_data = Board::new(5);
        test_data.set_cell(&Cell::new(0, 0), 2);
        test_data.set_cell(&Cell::new(0, 1), 2);
        test_data.set_cell(&Cell::new(0, 2), 2);
        test_data.set_cell(&Cell::new(0, 3), 2);
        test_data.set_cell(&Cell::new(1, 2), 2);
        test_data.set_cell(&Cell::new(2, 1), 2);

        test_game.bfs(Cell::new(0, 2), 1, 2);
        assert_eq!(test_game.data, test_data);

        test_data.set_cell(&Cell::new(4, 0), 2);
        test_data.set_cell(&Cell::new(3, 0), 2);

        test_game.bfs(Cell::new(3, 0), 2, 2);
        assert_eq!(test_game.data, test_data);
    }

    #[test]
    fn test_process_cell() {
        let mut test_game = GameWithData::new(accounts(0), accounts(1), 5, None);
        test_game.game.board.set_cell(&Cell::new(0, 0), 1);
        test_game.game.board.set_cell(&Cell::new(0, 1), 1);
        test_game.game.board.set_cell(&Cell::new(0, 2), 1);

        test_game.game.board.set_cell(&Cell::new(4, 4), 1);
        test_game.game.board.set_cell(&Cell::new(3, 4), 1);

        test_game.game.board.set_cell(&Cell::new(0, 3), 1);
        test_game.game.board.set_cell(&Cell::new(1, 2), 1);

        test_game.game.board.set_cell(&Cell::new(4, 0), 2);

        test_game.game.board.set_cell(&Cell::new(2, 1), 1);

        test_game.game.board.set_cell(&Cell::new(3, 0), 2);

        let mut test_data = Board::new(5);
        test_data.set_cell(&Cell::new(0, 0), 1);
        test_data.set_cell(&Cell::new(0, 1), 1);
        test_data.set_cell(&Cell::new(0, 2), 1);
        test_data.set_cell(&Cell::new(0, 3), 1);
        test_data.set_cell(&Cell::new(1, 2), 1);
        test_data.set_cell(&Cell::new(2, 1), 1);

        test_game.process_cell(Cell::new(0, 1));
        assert_eq!(test_game.data, Board::new(5));

        test_game.process_cell(Cell::new(0, 0));
        assert_eq!(test_game.data, test_data);
    }

    #[test]
    fn test_make_move() {
        let mut test_game = GameWithData::new(accounts(0), accounts(1), 5, None);
        let mut test_data = Board::new(5);
        assert_eq!(test_game.data, test_data);

        testing_env!(get_context(accounts(0)));
        test_game.make_move(MoveType::PLACE, Some(Cell::new(3, 0)));
        test_data.set_cell(&Cell::new(3, 0), 1);
        assert_eq!(test_game.data, test_data);

        testing_env!(get_context(accounts(1)));
        test_game.make_move(MoveType::SWAP, None);
        test_data.set_cell(&Cell::new(3, 0), 0);
        test_data.set_cell(&Cell::new(0, 3), 1);
        assert_eq!(test_game.data, test_data);

        testing_env!(get_context(accounts(0)));
        test_game.make_move(MoveType::PLACE, Some(Cell::new(4, 4)));
        test_data.set_cell(&Cell::new(4, 4), 2);
        assert_eq!(test_game.data, test_data);

        testing_env!(get_context(accounts(1)));
        test_game.make_move(MoveType::PLACE, Some(Cell::new(1, 2)));
        test_data.set_cell(&Cell::new(1, 2), 1);
        assert_eq!(test_game.data, test_data);

        testing_env!(get_context(accounts(0)));
        test_game.make_move(MoveType::PLACE, Some(Cell::new(4, 2)));
        assert_eq!(test_game.data, test_data);

        testing_env!(get_context(accounts(1)));
        test_game.make_move(MoveType::PLACE, Some(Cell::new(3, 2)));
        assert_eq!(test_game.data, test_data);

        testing_env!(get_context(accounts(0)));
        test_game.make_move(MoveType::PLACE, Some(Cell::new(4, 3)));
        test_data.set_cell(&Cell::new(4, 2), 2);
        test_data.set_cell(&Cell::new(4, 3), 2);
        assert_eq!(test_game.data, test_data);
    }

    #[test]
    #[should_panic]
    fn test_make_move_incorrect_args() {
        let mut test_game = GameWithData::new(accounts(0), accounts(1), 5, None);
        test_game.make_move(MoveType::PLACE, None);
    }

    #[test]
    #[should_panic]
    fn test_make_move_wrong_player() {
        let mut test_game = GameWithData::new(accounts(0), accounts(1), 5, None);
        testing_env!(get_context(accounts(1)));
        test_game.make_move(MoveType::PLACE, Some(Cell::new(0, 0)));
    }
}
