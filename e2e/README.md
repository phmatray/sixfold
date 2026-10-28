# Browser tests

Playwright drives Chromium through the main journeys (new game and the guided duel, a duel to
the end, packs, deck editing, the world, the finale after the Warden, keyboard-only runs of each,
axe-core on every screen, one journey at phone size) against
a release build. Assertions read what the player sees (roles, labels, live regions) and what the
game saved (`sixfold.save.<n>` in localStorage).

```bash
cd web && trunk build --release        # the suite serves web/dist
cd ../e2e && npm ci && npx playwright install chromium --only-shell
npx playwright test                    # all journeys, desktop and phone
npx playwright test world --headed     # one spec, in a visible browser
npx playwright show-report             # after a CI failure: download the e2e-report artifact first
```

`node perf-walkers.mjs` (after the release build) measures the frame with six people walking their
routines in Brightwell Square at night, on the phone viewport with the CPU slowed 4×, against the same
scene with nobody walking (#187). It opens installed Chrome in a visible window and only prints;
`--webkit` adds a WebKit run, `--runs N` and `--throttle N` change the defaults (5, 4).

`node life-clips.mjs [url]` records an ~8 s, 1920x1080, 60 fps MP4 of each region's ambient life (#190: critters,
perching birds, leaves, embers, Gullhaven's docking boat, the seal-site calm) by day with the HUD hidden, into
git-ignored `e2e/clips/`. Visible Chrome and ffmpeg needed; raw footage for AC7 of #115 and the beta trailer.
