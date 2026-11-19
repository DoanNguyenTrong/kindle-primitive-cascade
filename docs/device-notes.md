# Cascade device notes

Cascade is the working title. The app uses six shapes and a 7 × 13 board made for its e-ink layout. The local paces
start at 1,450 ms, 1,050 ms, and 820 ms per step and accelerate with cleared rows. A 500 ms software floor applies.
The device driver receives ordinary movement as partial changes and row clears and game over as full changes. The app
also asks for a full refresh after eight seconds of visible play when movement next changes the frame.

Before approving release timing, run the panel test in implementation plan A4.9 on the target Kindle. Check legibility
and ghosting for each pace with both orientations. Verify response to page buttons, turn, step, and drop, and whether
the interval between full updates is comfortable. Press sleep while a piece is falling and confirm that the game
requires Play after wake. The package is an ARMHF candidate for this test.
