use std::{
    fs, io,
    path::PathBuf,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use cascade_core::{Game, Outcome, Pace, Shape};
use primitive_app::{App, Change, WIDTH};
use primitive_ui::{
    back_button, header, screen_rect, Canvas, Input, Rect, TextStyle, BLACK, CAPTION, HAIRLINE,
    PAPER,
};

const TILE: i32 = 84;
const BOARD_X: i32 = 338;
const BOARD_Y: i32 = 236;
const FULL_REFRESH: Duration = Duration::from_secs(8);
const DIALOG: Rect = Rect::new(233, 532, 798, 560);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Dialog {
    Pace,
    Restart,
}

pub struct View {
    game: Game,
    best: u64,
    pace: Pace,
    data: Option<PathBuf>,
    deadline: Option<Instant>,
    last_full: Instant,
    dialog: Option<Dialog>,
    save_error: bool,
}

impl View {
    pub fn new(game: Game, best: u64, pace: Pace, data: Option<PathBuf>) -> Self {
        Self {
            best: best.max(game.score),
            game,
            pace,
            data,
            deadline: None,
            last_full: Instant::now(),
            dialog: None,
            save_error: false,
        }
    }

    pub fn load(path: PathBuf) -> Self {
        let (game, best, pace) = fs::read(&path)
            .ok()
            .and_then(|file| serde_json::from_slice::<(Game, u64, Pace)>(&file).ok())
            .filter(|(game, _, _)| game.is_valid())
            .unwrap_or_else(|| (Game::new(seed()), 0, Pace::default()));
        Self::new(game, best, pace, Some(path))
    }

    pub fn game(&self) -> &Game {
        &self.game
    }

    fn next_step(&mut self, now: Instant) {
        self.deadline = now.checked_add(Duration::from_millis(
            self.pace.interval_ms(self.game.level()),
        ));
    }

    fn record(&mut self) {
        self.save_error = self.save().is_err();
    }

    fn refresh(&mut self, now: Instant, full: bool) -> Change {
        if full || now.saturating_duration_since(self.last_full) >= FULL_REFRESH {
            self.last_full = now;
            Change::Full
        } else {
            Change::Partial
        }
    }

    fn applied(&mut self, outcome: Outcome, now: Instant) -> Change {
        self.best = self.best.max(self.game.score);
        match outcome {
            Outcome::Unchanged => Change::None,
            Outcome::Motion => self.refresh(now, false),
            Outcome::Settled => {
                self.next_step(now);
                self.record();
                self.refresh(now, false)
            }
            Outcome::Cleared(_) => {
                self.next_step(now);
                self.record();
                self.refresh(now, true)
            }
            Outcome::Ended => {
                self.deadline = None;
                self.record();
                self.refresh(now, true)
            }
        }
    }

    fn preview(&self, canvas: &mut Canvas, w: i32, x: i32, shape: Option<Shape>) {
        canvas.card(screen_rect(w, x, 342, 240, 225));
        if let Some(shape) = shape {
            let cells = shape.cells(0);
            let width = i32::from(cells.iter().map(|&(x, _)| x).max().unwrap_or(0)) + 1;
            let height = i32::from(cells.iter().map(|&(_, y)| y).max().unwrap_or(0)) + 1;
            let (left, top) = (x + (240 - width * 52) / 2, 342 + (225 - height * 52) / 2);
            for (cx, cy) in cells {
                stone(
                    canvas,
                    screen_rect(
                        w,
                        left + i32::from(cx) * 52,
                        top + i32::from(cy) * 52,
                        52,
                        52,
                    ),
                    shape,
                    false,
                );
            }
        } else {
            canvas.text(screen_rect(w, x, 426, 240, 60), "Empty", CAPTION.center());
        }
    }
}

fn seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

fn tile(w: i32, x: usize, y: usize) -> Rect {
    screen_rect(
        w,
        BOARD_X + x as i32 * TILE,
        BOARD_Y + y as i32 * TILE,
        TILE,
        TILE,
    )
}

fn stone(canvas: &mut Canvas, cell: Rect, shape: Shape, moving: bool) {
    let edge = (cell.w / 14).max(1);
    let tile = cell.inset(edge);
    canvas.fill(tile, BLACK);
    let inside = tile.inset((tile.w / 9).max(1));
    if moving {
        canvas.fill(inside, PAPER);
    }
    let ink = if moving { BLACK } else { PAPER };
    let mark = inside.inset((inside.w / 5).max(1));
    match shape.band() {
        0 => canvas.circle(mark, ink),
        1 => canvas.fill(
            Rect::new(mark.x + mark.w / 2, mark.y, (mark.w / 4).max(1), mark.h),
            ink,
        ),
        2 => canvas.fill(
            Rect::new(mark.x, mark.y + mark.h / 2, mark.w, (mark.h / 4).max(1)),
            ink,
        ),
        _ => {
            canvas.fill(
                Rect::new(mark.x + mark.w / 2, mark.y, (mark.w / 5).max(1), mark.h),
                ink,
            );
            canvas.fill(
                Rect::new(mark.x, mark.y + mark.h / 2, mark.w, (mark.h / 5).max(1)),
                ink,
            );
        }
    }
}

fn motion(w: i32, index: i32) -> Rect {
    screen_rect(w, 40 + index * 239, 1396, 228, 100)
}

fn action(w: i32, index: i32) -> Rect {
    screen_rect(w, 40 + index * 299, 1522, 285, 100)
}

fn dialog_choice(w: i32, index: i32) -> Rect {
    screen_rect(w, 268 + index * 247, 792, 228, 100)
}

fn cancel(w: i32) -> Rect {
    screen_rect(w, 508, 950, 248, 100)
}

impl App for View {
    fn draw(&self, canvas: &mut Canvas) {
        let w = canvas.bounds().w;
        header(canvas, "Cascade", "Exit");
        let message = if self.save_error {
            "Could not save this game."
        } else if self.game.ended {
            "The stack reached the top. Start a new game."
        } else if self.deadline.is_some() {
            "Fill a row to clear it. The page buttons move sideways."
        } else {
            "Paused. Tap Play to continue."
        };
        canvas.text(screen_rect(w, 40, 162, 1184, 55), message, CAPTION.center());
        canvas.text(
            screen_rect(w, 57, 275, 240, 55),
            "Next",
            TextStyle::new(37).bold().center(),
        );
        canvas.text(
            screen_rect(w, 968, 275, 240, 55),
            "Hold",
            TextStyle::new(37).bold().center(),
        );
        self.preview(canvas, w, 57, Some(self.game.next));
        self.preview(canvas, w, 968, self.game.held);
        for (x, y, label, value) in [
            (57, 645, "Score", self.game.score.to_string()),
            (57, 840, "Best", self.best.to_string()),
            (968, 645, "Level", self.game.level().to_string()),
            (968, 840, "Rows", self.game.rows.to_string()),
        ] {
            canvas.text(screen_rect(w, x, y, 240, 50), label, CAPTION.center());
            canvas.text(
                screen_rect(w, x, y + 53, 240, 70),
                &value,
                TextStyle::new(47).bold().center(),
            );
        }
        canvas.text(
            screen_rect(w, 968, 1054, 240, 80),
            self.pace.name(),
            CAPTION.center(),
        );
        for (y, row) in self.game.board().iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                let bounds = tile(w, x, y);
                canvas.fill(bounds, HAIRLINE);
                canvas.fill(bounds.inset(canvas.px(1)), PAPER);
                if let Some(shape) = cell {
                    stone(canvas, bounds, *shape, false);
                }
            }
        }
        if let Some(piece) = self.game.active() {
            for (x, y) in piece.cells() {
                stone(canvas, tile(w, x as usize, y as usize), piece.shape, true);
            }
        }
        for (index, label) in [
            (0, "Left"),
            (1, "Turn"),
            (2, "Right"),
            (3, "Step"),
            (4, "Drop"),
        ] {
            if self.deadline.is_some() {
                canvas.button(motion(w, index), label, index == 4, false);
            } else {
                canvas.text(
                    motion(w, index).inset(canvas.px(17)),
                    label,
                    CAPTION.center(),
                );
            }
        }
        let play = if self.game.ended {
            "Play again"
        } else if self.deadline.is_some() {
            "Pause"
        } else {
            "Play"
        };
        canvas.button(action(w, 0), play, true, false);
        if self.deadline.is_some() && self.game.hold_ready() {
            canvas.button(action(w, 1), "Hold piece", false, false);
        } else {
            canvas.text(
                action(w, 1).inset(canvas.px(17)),
                "Hold piece",
                CAPTION.center(),
            );
        }
        canvas.button(action(w, 2), "Pace", false, false);
        canvas.button(action(w, 3), "New game", false, false);
        if let Some(dialog) = self.dialog {
            canvas.popup(screen_rect(w, DIALOG.x, DIALOG.y, DIALOG.w, DIALOG.h));
            let (heading, message) = match dialog {
                Dialog::Pace => ("Choose a pace", "The pieces fall faster as you clear rows."),
                Dialog::Restart => ("Start a new game?", "The current game will be cleared."),
            };
            canvas.text(
                screen_rect(w, 275, 608, 714, 95),
                heading,
                TextStyle::new(50).bold().center(),
            );
            canvas.text(screen_rect(w, 275, 722, 714, 90), message, CAPTION.center());
            match dialog {
                Dialog::Pace => {
                    for (index, pace) in Pace::ALL.into_iter().enumerate() {
                        canvas.button(
                            dialog_choice(w, index as i32),
                            pace.name(),
                            pace == self.pace,
                            false,
                        );
                    }
                }
                Dialog::Restart => canvas.button(dialog_choice(w, 1), "Start", true, false),
            }
            canvas.button(cancel(w), "Cancel", false, false);
        }
    }

    fn input(&mut self, input: Input) -> Change {
        let now = Instant::now();
        let w = WIDTH as i32;
        match input {
            Input::Sleep => {
                self.deadline = None;
                return Change::None;
            }
            Input::Wake => return self.refresh(now, true),
            _ => {}
        }
        if let Some(dialog) = self.dialog {
            let Input::Tap(x, y) = input else {
                return Change::None;
            };
            if cancel(w).contains(x, y)
                || !screen_rect(w, DIALOG.x, DIALOG.y, DIALOG.w, DIALOG.h).contains(x, y)
            {
                self.dialog = None;
                return self.refresh(now, true);
            }
            match dialog {
                Dialog::Pace => {
                    for (index, pace) in Pace::ALL.into_iter().enumerate() {
                        if dialog_choice(w, index as i32).contains(x, y) {
                            self.pace = pace;
                            self.dialog = None;
                            self.record();
                            return self.refresh(now, true);
                        }
                    }
                }
                Dialog::Restart if dialog_choice(w, 1).contains(x, y) => {
                    self.game = Game::new(seed());
                    self.dialog = None;
                    self.record();
                    return self.refresh(now, true);
                }
                Dialog::Restart => {}
            }
            return Change::None;
        }
        if let Input::Tap(x, y) = input {
            if back_button(w).contains(x, y) {
                return Change::Exit;
            }
            if action(w, 0).contains(x, y) {
                if self.deadline.take().is_some() {
                    self.record();
                } else {
                    if self.game.ended {
                        self.game = Game::new(seed());
                    }
                    self.next_step(now);
                }
                return self.refresh(now, true);
            }
            for (index, dialog) in [(2, Dialog::Pace), (3, Dialog::Restart)] {
                if action(w, index).contains(x, y) {
                    self.deadline = None;
                    self.dialog = Some(dialog);
                    self.record();
                    return self.refresh(now, true);
                }
            }
        }
        if self.deadline.is_none() {
            return Change::None;
        }
        let outcome = match input {
            Input::PageBack => self.game.shift(-1),
            Input::PageForward => self.game.shift(1),
            Input::Tap(x, y) if motion(w, 0).contains(x, y) => self.game.shift(-1),
            Input::Tap(x, y) if motion(w, 1).contains(x, y) => self.game.rotate(),
            Input::Tap(x, y) if motion(w, 2).contains(x, y) => self.game.shift(1),
            Input::Tap(x, y) if motion(w, 3).contains(x, y) => self.game.step(true),
            Input::Tap(x, y) if motion(w, 4).contains(x, y) => self.game.drop(),
            Input::Tap(x, y) if action(w, 1).contains(x, y) => self.game.hold(),
            _ => Outcome::Unchanged,
        };
        self.applied(outcome, now)
    }

    fn deadline(&self) -> Option<Instant> {
        self.deadline
    }

    fn tick(&mut self, now: Instant) -> Change {
        if self.deadline.is_none_or(|next| now < next) {
            return Change::None;
        }
        self.next_step(now);
        let moved = self.game.step(false);
        self.applied(moved, now)
    }

    fn save(&mut self) -> io::Result<()> {
        if let Some(data) = &self.data {
            primitive_app::save(
                data,
                &serde_json::to_vec(&(&self.game, self.best, self.pace))?,
            )?;
        }
        Ok(())
    }
}
