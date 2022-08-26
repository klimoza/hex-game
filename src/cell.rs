use crate::*;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct Cell {
    pub x: usize,
    pub y: usize,
}

impl Cell {
    pub fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }

    pub fn get_neighbours(&self, field_size: usize) -> Vec<Cell> {
        let mut neighbours: Vec<Cell> = Vec::new();
        let (x, y) = (self.x, self.y);
        if self.x > 0 {
            neighbours.push(Cell::new(x - 1, y));
        }
        if self.y > 0 {
            neighbours.push(Cell::new(x, y - 1));
        }
        if self.x + 1 < field_size && self.y > 0 {
            neighbours.push(Cell::new(x + 1, y - 1));
        }
        if self.x + 1 < field_size {
            neighbours.push(Cell::new(x + 1, y));
        }
        if self.y + 1 < field_size {
            neighbours.push(Cell::new(x, y + 1));
        }
        if self.x > 0 && self.y + 1 < field_size {
            neighbours.push(Cell::new(x - 1, y + 1));
        }
        return neighbours;
    }

    pub fn symm(&self) -> Self {
        Self {
            x: self.y,
            y: self.x,
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod cell_tests {
    use super::Cell;

    impl PartialEq for Cell {
        fn eq(&self, other: &Self) -> bool {
            self.x == other.x && self.y == other.y
        }
    }

    #[test]
    fn test_cell_neighbours_single() {
        let test_cell = Cell::new(0, 0);
        let neighbours = test_cell.get_neighbours(1);
        assert!(neighbours.is_empty());
    }

    #[test]
    fn test_cell_neighbours_center() {
        let test_cell = Cell::new(1, 1);
        let neighbours = test_cell.get_neighbours(3);
        assert_eq!(
            neighbours,
            vec![
                Cell::new(0, 1),
                Cell::new(1, 0),
                Cell::new(2, 0),
                Cell::new(2, 1),
                Cell::new(1, 2),
                Cell::new(0, 2)
            ]
        );
    }
    #[test]
    fn test_cell_neighbours_right_bottom_corner() {
        let test_cell = Cell::new(1, 1);
        let neighbours = test_cell.get_neighbours(2);
        assert_eq!(neighbours, vec![Cell::new(0, 1), Cell::new(1, 0)]);
    }
    #[test]
    fn test_cell_neighbours_left_upper_corner() {
        let test_cell = Cell::new(0, 0);
        let neighbours = test_cell.get_neighbours(4);
        assert_eq!(neighbours, vec![Cell::new(1, 0), Cell::new(0, 1)]);
    }
    #[test]
    fn test_cell_neighbours_left_bottom_corner() {
        let test_cell = Cell::new(0, 2);
        let neighbours = test_cell.get_neighbours(3);
        assert_eq!(
            neighbours,
            vec![Cell::new(0, 1), Cell::new(1, 1), Cell::new(1, 2)]
        );
    }
    #[test]
    fn test_cell_neighbours_right_upper_corner() {
        let test_cell = Cell::new(4, 0);
        let neighbours = test_cell.get_neighbours(5);
        assert_eq!(
            neighbours,
            vec![Cell::new(3, 0), Cell::new(4, 1), Cell::new(3, 1)]
        );
    }

    #[test]
    fn test_cell_neighbours_left_border() {
        let test_cell = Cell::new(0, 1);
        let neighbours = test_cell.get_neighbours(3);
        assert_eq!(
            neighbours,
            vec![
                Cell::new(0, 0),
                Cell::new(1, 0),
                Cell::new(1, 1),
                Cell::new(0, 2)
            ]
        );
    }

    #[test]
    fn test_cell_neighbours_upper_border() {
        let test_cell = Cell::new(2, 0);
        let neighbours = test_cell.get_neighbours(4);
        assert_eq!(
            neighbours,
            vec![
                Cell::new(1, 0),
                Cell::new(3, 0),
                Cell::new(2, 1),
                Cell::new(1, 1)
            ]
        );
    }

    #[test]
    fn test_cell_neighbours_right_border() {
        let test_cell = Cell::new(4, 3);
        let neighbours = test_cell.get_neighbours(5);
        assert_eq!(
            neighbours,
            vec![
                Cell::new(3, 3),
                Cell::new(4, 2),
                Cell::new(4, 4),
                Cell::new(3, 4)
            ]
        );
    }

    #[test]
    fn test_cell_neighbours_bottom_border() {
        let test_cell = Cell::new(2, 5);
        let neighbours = test_cell.get_neighbours(6);
        assert_eq!(
            neighbours,
            vec![
                Cell::new(1, 5),
                Cell::new(2, 4),
                Cell::new(3, 4),
                Cell::new(3, 5)
            ]
        );
    }

    #[test]
    #[should_panic]
    fn test_cell_of_bounds() {
        Cell::new(3, 4).get_neighbours(4);
    }
}
