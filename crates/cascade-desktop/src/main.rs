use std::{
    fs,
    path::{Path, PathBuf},
};

use cascade_core::{Game, Pace};
use cascade_render::View;
use primitive_app::{font_paths, frame, write_png, App, Change, HEIGHT, WIDTH};
use primitive_ui::{Canvas, Input};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .unwrap_or_else(|| "build/previews".into()),
    );
    fs::create_dir_all(&out)?;
    let fonts =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../doannt-kindle-launcher/assets/fonts");
    let mut canvas = Canvas::new(WIDTH, HEIGHT, &font_paths(&fonts))?;
    let mut app = View::new(Game::new(42), 0, Pace::Steady, None);
    write_png(
        &out.join("cascade-paused.png"),
        WIDTH,
        HEIGHT,
        &frame(&app, &mut canvas, false),
    )?;
    assert_eq!(app.input(Input::Tap(160.0, 1565.0)), Change::Full);
    assert_eq!(app.input(Input::PageBack), Change::Partial);
    assert_eq!(app.input(Input::Tap(460.0, 1570.0)), Change::Partial);
    assert!(app.game().held.is_some());
    let next = app.deadline().expect("falling deadline");
    assert_eq!(app.tick(next), Change::Partial);
    assert_eq!(app.input(Input::Tap(1120.0, 1443.0)), Change::Partial);
    assert!(app.game().score > 0);
    write_png(
        &out.join("cascade-dark.png"),
        WIDTH,
        HEIGHT,
        &frame(&app, &mut canvas, true),
    )?;
    assert_eq!(app.input(Input::Sleep), Change::None);
    assert_eq!(app.deadline(), None);
    assert_eq!(app.input(Input::Wake), Change::Full);
    assert_eq!(app.input(Input::Tap(740.0, 1558.0)), Change::Full);
    write_png(
        &out.join("cascade-pace.png"),
        WIDTH,
        HEIGHT,
        &frame(&app, &mut canvas, false),
    )?;
    Ok(())
}
