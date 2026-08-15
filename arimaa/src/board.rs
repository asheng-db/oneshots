//! Board model for Arimaa: pieces, cells, traps, freeze/capture/goal logic.

pub const SIZE: usize = 8;

// Trap squares: c3, f3, c6, f6 as (row, col) 0-indexed.
pub const TRAPS: [[usize; 2]; 4] = [[2, 2], [2, 5], [5, 2], [5, 5]];

/// Whether (r,c) is a trap square.
#[allow(dead_code)]
pub fn is_trap(r: usize, c: usize) -> bool {
    TRAPS.iter().any(|t| t[0] == r && t[1] == c)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Color {
    Gold,
    Silver,
}

impl Color {
    pub fn other(self) -> Color {
        match self {
            Color::Gold => Color::Silver,
            Color::Silver => Color::Gold,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Color::Gold => "Gold",
            Color::Silver => "Silver",
        }
    }
}

// Strongest -> weakest. Higher value = stronger.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PType {
    Elephant = 6,
    Camel = 5,
    Horse = 4,
    Dog = 3,
    Cat = 2,
    Rabbit = 1,
}

impl PType {
    pub fn letter(self) -> char {
        match self {
            PType::Elephant => 'E',
            PType::Camel => 'M',
            PType::Horse => 'H',
            PType::Dog => 'D',
            PType::Cat => 'C',
            PType::Rabbit => 'R',
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Piece {
    pub color: Color,
    pub ptype: PType,
}

impl Piece {
    pub fn strength(self) -> i32 {
        self.ptype as i32
    }
    // Display letter: uppercase for Gold, lowercase for Silver.
    pub fn letter(self) -> char {
        let l = self.ptype.letter();
        match self.color {
            Color::Gold => l,
            Color::Silver => l.to_ascii_lowercase(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cell {
    Empty,
    Piece(Piece),
}

#[derive(Clone, Debug)]
pub struct Board {
    pub cells: [[Cell; SIZE]; SIZE],
}

impl Board {
    pub fn new() -> Board {
        Board {
            cells: [[Cell::Empty; SIZE]; SIZE],
        }
    }

    pub fn at(&self, r: usize, c: usize) -> Cell {
        self.cells[r][c]
    }

    pub fn set(&mut self, r: usize, c: usize, cell: Cell) {
        self.cells[r][c] = cell;
    }

    // Fixed setup based on Wikipedia Diagram 1.
    pub fn setup(&mut self) {
        // Gold home rows: rank 1 (row 0) and rank 2 (row 1).
        let gold = [
            (0, 0, PType::Cat),
            (0, 1, PType::Dog),
            (0, 2, PType::Horse),
            (0, 3, PType::Camel),
            (0, 4, PType::Elephant),
            (0, 5, PType::Rabbit),
            (0, 6, PType::Rabbit),
            (0, 7, PType::Rabbit),
            (1, 0, PType::Cat),
            (1, 1, PType::Dog),
            (1, 2, PType::Horse),
            (1, 3, PType::Rabbit),
            (1, 4, PType::Rabbit),
            (1, 5, PType::Rabbit),
            (1, 6, PType::Rabbit),
            (1, 7, PType::Rabbit),
        ];
        for (r, c, pt) in gold {
            self.set(r, c, Cell::Piece(Piece { color: Color::Gold, ptype: pt }));
        }
        // Silver home rows: rank 7 (row 6) and rank 8 (row 7).
        let silver = [
            (6, 0, PType::Horse),
            (6, 1, PType::Dog),
            (6, 2, PType::Cat),
            (6, 3, PType::Elephant),
            (6, 4, PType::Camel),
            (6, 5, PType::Dog),
            (6, 6, PType::Cat),
            (6, 7, PType::Horse),
            (7, 0, PType::Rabbit),
            (7, 1, PType::Rabbit),
            (7, 2, PType::Rabbit),
            (7, 3, PType::Rabbit),
            (7, 4, PType::Rabbit),
            (7, 5, PType::Rabbit),
            (7, 6, PType::Rabbit),
            (7, 7, PType::Rabbit),
        ];
        for (r, c, pt) in silver {
            self.set(r, c, Cell::Piece(Piece { color: Color::Silver, ptype: pt }));
        }
    }

    pub fn piece_at(&self, r: usize, c: usize) -> Option<Piece> {
        match self.at(r, c) {
            Cell::Piece(p) => Some(p),
            Cell::Empty => None,
        }
    }

    pub fn count_rabbits(&self, color: Color) -> usize {
        let mut n = 0;
        for r in 0..SIZE {
            for c in 0..SIZE {
                if let Cell::Piece(p) = self.at(r, c) {
                    if p.color == color && p.ptype == PType::Rabbit {
                        n += 1;
                    }
                }
            }
        }
        n
    }

    pub fn count_pieces(&self, color: Color) -> usize {
        let mut n = 0;
        for r in 0..SIZE {
            for c in 0..SIZE {
                if let Cell::Piece(p) = self.at(r, c) {
                    if p.color == color {
                        n += 1;
                    }
                }
            }
        }
        n
    }

    // A piece is frozen if adjacent to a stronger enemy piece and no friendly
    // piece is adjacent. Elephant cannot be frozen.
    pub fn is_frozen(&self, r: usize, c: usize) -> bool {
        let p = match self.piece_at(r, c) {
            Some(p) => p,
            None => return false,
        };
        if p.ptype == PType::Elephant {
            return false;
        }
        let has_friendly_adj = neighbors(r, c).iter().any(|&(nr, nc)| {
            matches!(self.piece_at(nr, nc), Some(q) if q.color == p.color)
        });
        if has_friendly_adj {
            return false;
        }
        neighbors(r, c).iter().any(|&(nr, nc)| {
            matches!(self.piece_at(nr, nc), Some(q) if q.color == p.color.other() && q.strength() > p.strength())
        })
    }

    // Remove any piece sitting on a trap square that has no friendly support.
    pub fn apply_traps(&mut self) {
        for &[r, c] in TRAPS.iter() {
            if let Cell::Piece(p) = self.at(r, c) {
                let supported = neighbors(r, c).iter().any(|&(nr, nc)| {
                    matches!(self.piece_at(nr, nc), Some(q) if q.color == p.color)
                });
                if !supported {
                    self.set(r, c, Cell::Empty);
                }
            }
        }
    }

    // Goal check: a rabbit of `color` on the opponent's home rank.
    // Gold rabbits reach rank 8 (row 7); Silver rabbits reach rank 1 (row 0).
    pub fn goal_for(&self, color: Color) -> bool {
        let rank = match color {
            Color::Gold => 7,
            Color::Silver => 0,
        };
        for c in 0..SIZE {
            if let Cell::Piece(p) = self.at(rank, c) {
                if p.color == color && p.ptype == PType::Rabbit {
                    return true;
                }
            }
        }
        false
    }
}

// Orthogonal neighbor coordinates within the board.
pub fn neighbors(r: usize, c: usize) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    if r > 0 { out.push((r - 1, c)); }
    if r + 1 < SIZE { out.push((r + 1, c)); }
    if c > 0 { out.push((r, c - 1)); }
    if c + 1 < SIZE { out.push((r, c + 1)); }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trap_squares_exist() {
        assert!(is_trap(2, 2));
        assert!(is_trap(2, 5));
        assert!(is_trap(5, 2));
        assert!(is_trap(5, 5));
        assert!(!is_trap(0, 0));
    }

    #[test]
    fn piece_letters() {
        let g = Piece { color: Color::Gold, ptype: PType::Elephant };
        let s = Piece { color: Color::Silver, ptype: PType::Rabbit };
        assert_eq!(g.letter(), 'E');
        assert_eq!(s.letter(), 'r');
    }

    #[test]
    fn initial_setup_counts() {
        let mut b = Board::new();
        b.setup();
        assert_eq!(b.count_pieces(Color::Gold), 16);
        assert_eq!(b.count_pieces(Color::Silver), 16);
        assert_eq!(b.count_rabbits(Color::Gold), 8);
        assert_eq!(b.count_rabbits(Color::Silver), 8);
        assert!(!b.goal_for(Color::Gold));
        assert!(!b.goal_for(Color::Silver));
    }
}
