//! Cascade's falling-piece rules, piece queue, hold, and scores.

use serde::{Deserialize, Serialize};

pub const LANES: usize = 7;
pub const DEPTH: usize = 13;
type Board = [[Option<Shape>; LANES]; DEPTH];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Shape {
    Seed,
    Skip,
    Hook,
    Rail,
    Arch,
    Sprig,
}

impl Shape {
    const ALL: [Self; 6] = [
        Self::Seed,
        Self::Skip,
        Self::Hook,
        Self::Rail,
        Self::Arch,
        Self::Sprig,
    ];

    pub fn cells(self, turn: u8) -> Vec<(i8, i8)> {
        let pattern: &[(i8, i8)] = match self {
            Self::Seed => &[(0, 0)],
            Self::Skip => &[(0, 0), (1, 0)],
            Self::Hook => &[(0, 0), (1, 0), (1, 1)],
            Self::Rail => &[(0, 0), (1, 0), (2, 0)],
            Self::Arch => &[(0, 0), (1, 0), (2, 0), (0, 1), (2, 1)],
            Self::Sprig => &[(1, 0), (0, 1), (1, 1), (1, 2), (2, 2)],
        };
        let mut cells = pattern.to_vec();
        for _ in 0..turn % 4 {
            for (x, y) in &mut cells {
                (*x, *y) = (-*y, *x);
            }
        }
        let left = cells.iter().map(|(x, _)| *x).min().unwrap_or_default();
        let top = cells.iter().map(|(_, y)| *y).min().unwrap_or_default();
        for (x, y) in &mut cells {
            *x -= left;
            *y -= top;
        }
        cells
    }

    pub fn band(self) -> u8 {
        match self {
            Self::Seed => 0,
            Self::Skip | Self::Hook => 1,
            Self::Rail => 2,
            Self::Arch | Self::Sprig => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Piece {
    pub shape: Shape,
    x: i8,
    y: i8,
    turn: u8,
}

impl Piece {
    fn new(shape: Shape) -> Self {
        let width = shape
            .cells(0)
            .iter()
            .map(|&(x, _)| usize::try_from(x).unwrap_or_default() + 1)
            .max()
            .unwrap_or(1);
        Self {
            shape,
            x: ((LANES - width) / 2) as i8,
            y: 0,
            turn: 0,
        }
    }

    pub fn cells(self) -> impl Iterator<Item = (i8, i8)> {
        self.shape
            .cells(self.turn)
            .into_iter()
            .map(move |(x, y)| (x.saturating_add(self.x), y.saturating_add(self.y)))
    }

    fn translate(self, dx: i8, dy: i8) -> Self {
        Self {
            x: self.x.saturating_add(dx),
            y: self.y.saturating_add(dy),
            ..self
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Pace {
    Gentle,
    #[default]
    Steady,
    Brisk,
}

impl Pace {
    pub const ALL: [Self; 3] = [Self::Gentle, Self::Steady, Self::Brisk];

    pub fn name(self) -> &'static str {
        match self {
            Self::Gentle => "Gentle",
            Self::Steady => "Steady",
            Self::Brisk => "Brisk",
        }
    }

    pub fn interval_ms(self, level: u32) -> u64 {
        let base: u64 = match self {
            Self::Gentle => 1450,
            Self::Steady => 1050,
            Self::Brisk => 820,
        };
        base.saturating_sub(u64::from(level.saturating_sub(1)) * 75)
            .max(500)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Unchanged,
    Motion,
    Settled,
    Cleared(u8),
    Ended,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Game {
    board: Board,
    active: Piece,
    pub next: Shape,
    pub held: Option<Shape>,
    pub score: u64,
    pub rows: u32,
    pub ended: bool,
    held_this_piece: bool,
    random: u64,
    queue: [Shape; 6],
    queue_index: usize,
}

impl Game {
    pub fn new(seed: u64) -> Self {
        let mut game = Self {
            board: Board::default(),
            active: Piece::new(Shape::Seed),
            next: Shape::Seed,
            held: None,
            score: 0,
            rows: 0,
            ended: false,
            held_this_piece: false,
            random: seed.max(1),
            queue: Shape::ALL,
            queue_index: Shape::ALL.len(),
        };
        game.active = Piece::new(game.draw_shape());
        game.next = game.draw_shape();
        game
    }

    pub fn is_valid(&self) -> bool {
        self.random != 0
            && self.queue_index <= self.queue.len()
            && Shape::ALL.iter().all(|shape| self.queue.contains(shape))
            && self.active.turn < 4
            && (self.ended || self.fits(self.active))
            && self.board.iter().all(|row| row.iter().any(Option::is_none))
    }

    pub fn level(&self) -> u32 {
        self.rows / 3 + 1
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn active(&self) -> Option<Piece> {
        (!self.ended).then_some(self.active)
    }

    pub fn hold_ready(&self) -> bool {
        !self.ended && !self.held_this_piece
    }

    pub fn shift(&mut self, direction: i8) -> Outcome {
        if direction == 0 {
            return Outcome::Unchanged;
        }
        self.move_piece(self.active.translate(direction.signum(), 0))
    }

    pub fn rotate(&mut self) -> Outcome {
        let rotated = Piece {
            turn: (self.active.turn + 1) % 4,
            ..self.active
        };
        for (dx, dy) in [(0, 0), (-1, 0), (1, 0), (-2, 0), (2, 0), (0, -1)] {
            let moved = self.move_piece(rotated.translate(dx, dy));
            if moved == Outcome::Motion {
                return moved;
            }
        }
        Outcome::Unchanged
    }

    pub fn step(&mut self, manual: bool) -> Outcome {
        if self.ended {
            return Outcome::Unchanged;
        }
        if self.move_piece(self.active.translate(0, 1)) == Outcome::Motion {
            if manual {
                self.score = self.score.saturating_add(1);
            }
            Outcome::Motion
        } else {
            self.land()
        }
    }

    pub fn drop(&mut self) -> Outcome {
        if self.ended {
            return Outcome::Unchanged;
        }
        while self.move_piece(self.active.translate(0, 1)) == Outcome::Motion {
            self.score = self.score.saturating_add(2);
        }
        self.land()
    }

    pub fn hold(&mut self) -> Outcome {
        if !self.hold_ready() {
            return Outcome::Unchanged;
        }
        self.held_this_piece = true;
        let shape = self
            .held
            .replace(self.active.shape)
            .unwrap_or_else(|| self.draw_next());
        self.spawn(shape)
    }

    fn fits(&self, piece: Piece) -> bool {
        piece.cells().all(|(x, y)| {
            x >= 0
                && y >= 0
                && (x as usize) < LANES
                && (y as usize) < DEPTH
                && self.board[y as usize][x as usize].is_none()
        })
    }

    fn move_piece(&mut self, piece: Piece) -> Outcome {
        if self.ended || piece == self.active || !self.fits(piece) {
            return Outcome::Unchanged;
        }
        self.active = piece;
        Outcome::Motion
    }

    fn draw_shape(&mut self) -> Shape {
        if self.queue_index == self.queue.len() {
            for index in (1..self.queue.len()).rev() {
                self.random ^= self.random << 13;
                self.random ^= self.random >> 7;
                self.random ^= self.random << 17;
                self.queue
                    .swap(index, (self.random % (index + 1) as u64) as usize);
            }
            self.queue_index = 0;
        }
        let shape = self.queue[self.queue_index];
        self.queue_index += 1;
        shape
    }

    fn draw_next(&mut self) -> Shape {
        let shape = self.next;
        self.next = self.draw_shape();
        shape
    }

    fn spawn(&mut self, shape: Shape) -> Outcome {
        self.active = Piece::new(shape);
        if self.fits(self.active) {
            Outcome::Motion
        } else {
            self.ended = true;
            Outcome::Ended
        }
    }

    fn land(&mut self) -> Outcome {
        let shape = self.active.shape;
        for (x, y) in self.active.cells() {
            self.board[y as usize][x as usize] = Some(shape);
        }
        self.score = self.score.saturating_add(shape.cells(0).len() as u64);
        let landed = self.clear_rows();
        self.held_this_piece = false;
        let next = self.draw_next();
        if self.spawn(next) == Outcome::Ended {
            Outcome::Ended
        } else if landed == 0 {
            Outcome::Settled
        } else {
            Outcome::Cleared(landed)
        }
    }

    fn clear_rows(&mut self) -> u8 {
        let mut compact = Board::default();
        let mut write = DEPTH;
        for row in self
            .board
            .into_iter()
            .rev()
            .filter(|row| row.iter().any(Option::is_none))
        {
            write -= 1;
            compact[write] = row;
        }
        self.board = compact;
        self.rows = self.rows.saturating_add(write as u32);
        self.score = self.score.saturating_add(80 * (write as u64).pow(2));
        write as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movement_rotation_spawn_and_hold_use_the_active_board() {
        let mut game = Game::new(42);
        let original = game.active().unwrap().shape;
        assert_eq!(game.hold(), Outcome::Motion);
        assert_eq!(game.held, Some(original));
        assert_eq!(game.hold(), Outcome::Unchanged);
        while game.shift(-1) == Outcome::Motion {}
        assert_eq!(game.shift(-1), Outcome::Unchanged);
        game.rotate();
        assert!(game.is_valid());
        assert!(matches!(
            game.drop(),
            Outcome::Settled | Outcome::Cleared(_)
        ));
        assert!(game.hold_ready());
        assert!(game.score > 0);
    }

    #[test]
    fn a_completed_row_scores_and_shifts_above_tiles_down() {
        let mut game = Game::new(11);
        game.board[DEPTH - 1] = [Some(Shape::Seed); LANES];
        game.board[DEPTH - 2][0] = Some(Shape::Rail);
        assert_eq!(game.clear_rows(), 1);
        assert_eq!(game.board[DEPTH - 1][0], Some(Shape::Rail));
        assert_eq!(game.board[DEPTH - 2], [None; LANES]);
        assert_eq!(game.rows, 1);
        assert_eq!(game.score, 80);
    }

    #[test]
    fn crowded_spawn_ends_play_and_a_faster_level_obeys_the_floor() {
        let mut game = Game::new(8);
        game.board[0][LANES / 2] = Some(Shape::Seed);
        assert_eq!(game.spawn(Shape::Seed), Outcome::Ended);
        assert_eq!(game.step(false), Outcome::Unchanged);
        assert!(Pace::Brisk.interval_ms(1) < Pace::Gentle.interval_ms(1));
        assert_eq!(Pace::Steady.interval_ms(250), 500);
    }
}
