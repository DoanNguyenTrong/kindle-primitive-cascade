# Primitive Cascade

Cascade is the working title for an original falling-piece game for e-ink. Fit six shapes across a seven-lane board to
clear rows, earn points, and reach higher levels. Use the page buttons or touch controls to steer. **Turn** rotates the
piece, **Step** moves it down one row, and **Drop** places it immediately. Store one piece with **Hold piece** and see the
upcoming shape beside the board.

The game has three provisional paces, remembers your best score, and pauses when the Kindle sleeps. The release name
and device timing are pending panel review.

## Develop locally

Run `cargo test --offline --workspace` and `cargo run --offline -p cascade-desktop` with Primitive Tools and Launcher
beside this repository. The desktop target simulates falling steps, touch controls, and sleep and writes previews in
`build/previews`. See [device notes](docs/device-notes.md) for physical testing.
