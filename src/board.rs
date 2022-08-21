use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::Base64VecU8;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, require};

use crate::cell::Cell;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct Board {
    pub size: usize,
    pub field: Base64VecU8,
}

impl Board {
    pub fn new(size: usize) -> Self {
        require!(size <= 19, "The size of the field must be less or equal 19");
        let field_len = (size * size + 3) / 4;
        Board {
            size,
            field: Base64VecU8::from(vec![0u8; field_len]),
        }
    }

    fn get_byte_and_bit(&self, cell: &Cell) -> (u8, usize, usize) {
        let index = (self.size * cell.y + cell.x) * 2;
        let byte_index = index / 8;
        let byte: u8 = self.field.0[byte_index];
        let bit_index = index & 7;
        (byte, byte_index, bit_index)
    }

    pub fn get_cell(&self, cell: &Cell) -> u8 {
        require!(
            cell.x < self.size && cell.y < self.size,
            "Cell is out of bounds."
        );
        let (byte, _, bit_index) = self.get_byte_and_bit(cell);
        (byte >> bit_index) & 3
    }

    pub fn set_cell(&mut self, cell: &Cell, value: u8) {
        require!(
            cell.x < self.size && cell.y < self.size,
            "Cell is out of bounds."
        );
        require!(value <= 2, "Value is too big.");
        let (byte, byte_index, bit_index) = self.get_byte_and_bit(cell);
        let bits = (byte >> bit_index) & 3;
        let new_byte = byte ^ (bits << bit_index) ^ (value << bit_index);
        self.field.0[byte_index] = new_byte;
    }

    pub fn get_coords(&self, bit_number: usize) -> Cell {
        Cell::new(bit_number / 2 % self.size, bit_number / 2 / self.size)
    }

    pub fn get_board_as_strings(&self) -> Vec<String> {
        let mut vector = Vec::new();

        for i in 0..self.size {
            let mut result: String = (0..i).into_iter().map(|_| ' ').collect();
            for j in 0..self.size {
                let symbol = match self.get_cell(&Cell::new(j, i)) {
                    0 => '.',
                    1 => 'R',
                    2 => 'B',
                    _ => unreachable!(),
                };
                result.push(symbol);
                if j + 1 != self.size {
                    result.push(' ');
                }
            }
            vector.push(result);
        }

        vector
    }

    pub fn debug_logs(&self) {
        self.get_board_as_strings()
            .into_iter()
            .for_each(|s| env::log_str(&s));
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod board_tests {
    use near_sdk::json_types::Base64VecU8;

    use crate::cell::Cell;

    use super::Board;

    #[test]
    #[should_panic]
    fn test_board_is_too_big() {
        Board::new(20);
    }

    #[test]
    fn test_get_byte_and_bit() {
        let mut test_board = Board::new(11);
        assert_eq!((0, 3, 6), test_board.get_byte_and_bit(&Cell::new(4, 1)));
        assert_eq!((0, 0, 0), test_board.get_byte_and_bit(&Cell::new(0, 0)));
        assert_eq!((0, 5, 0), test_board.get_byte_and_bit(&Cell::new(9, 1)));

        let mut test_vec = vec![0u8; 7];
        test_vec[0] = 255;
        test_vec[1] = 19;
        test_vec[2] = 7;
        test_vec[3] = 113;
        test_board = Board {
            size: 5,
            field: Base64VecU8::from(test_vec),
        };

        assert_eq!((255, 0, 6), test_board.get_byte_and_bit(&Cell::new(3, 0)));
        assert_eq!((7, 2, 6), test_board.get_byte_and_bit(&Cell::new(1, 2)));
        assert_eq!((19, 1, 0), test_board.get_byte_and_bit(&Cell::new(4, 0)));
        assert_eq!((0, 6, 0), test_board.get_byte_and_bit(&Cell::new(4, 4)));
    }

    #[test]
    #[should_panic]
    fn test_get_cell_out_of_bounds() {
        Board::new(11).get_cell(&Cell::new(5, 11));
    }

    #[test]
    fn test_get_cell() {
        let mut test_vec = vec![0u8; 7];
        test_vec[0] = 255;
        test_vec[1] = 32;
        test_vec[2] = 7;
        test_vec[3] = 113;
        let test_board = Board {
            size: 5,
            field: Base64VecU8::from(test_vec),
        };

        assert_eq!(3, test_board.get_cell(&Cell::new(3, 0)));
        assert_eq!(1, test_board.get_cell(&Cell::new(4, 1)));
        assert_eq!(3, test_board.get_cell(&Cell::new(4, 2)));
        assert_eq!(2, test_board.get_cell(&Cell::new(1, 1)));
    }

    #[test]
    #[should_panic]
    fn test_set_cell_out_of_bounds() {
        Board::new(5).set_cell(&Cell::new(6, 3), 1);
    }

    #[test]
    #[should_panic]
    fn test_set_cell_too_big_value() {
        Board::new(4).set_cell(&Cell::new(1, 1), 3);
    }

    #[test]
    fn test_set_sell() {
        let mut test_board = Board::new(11);
        let test_cell = Cell::new(2, 5);

        test_board.set_cell(&test_cell, 1);
        assert_eq!(1, test_board.get_cell(&test_cell));

        test_board.set_cell(&test_cell, 0);
        assert_eq!(0, test_board.get_cell(&test_cell));

        test_board.set_cell(&test_cell, 2);
        assert_eq!(2, test_board.get_cell(&test_cell));

        test_board.set_cell(&Cell::new(10, 10), 1);
        test_board.set_cell(&Cell::new(3, 5), 1);

        assert_eq!(2, test_board.get_cell(&Cell::new(2, 5)));
        assert_eq!(1, test_board.get_cell(&Cell::new(10, 10)));
        assert_eq!(1, test_board.get_cell(&Cell::new(3, 5)));
    }

    #[test]
    fn test_get_coords() {
        let mut test_board = Board::new(5);
        assert_eq!(Cell::new(0, 1), test_board.get_coords(10));
        assert_eq!(Cell::new(0, 0), test_board.get_coords(0));
        assert_eq!(Cell::new(0, 0), test_board.get_coords(1));
        assert_eq!(Cell::new(4, 4), test_board.get_coords(48));

        test_board = Board::new(15);
        let mut test_cell = Cell::new(11, 7);
        let (_, mut byte, mut bit) = test_board.get_byte_and_bit(&test_cell);
        assert_eq!(test_cell, test_board.get_coords(byte * 8 + bit));

        test_cell = Cell::new(0, 14);
        (_, byte, bit) = test_board.get_byte_and_bit(&test_cell);
        assert_eq!(test_cell, test_board.get_coords(byte * 8 + bit));

        test_cell = Cell::new(10, 2);
        (_, byte, bit) = test_board.get_byte_and_bit(&test_cell);
        assert_eq!(test_cell, test_board.get_coords(byte * 8 + bit));
    }
}
