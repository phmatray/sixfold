# Browser tests

Playwright drives Chromium through the main journeys (new game and the guided duel, a duel to
the end, packs, deck editing, the world, keyboard-only runs of each, axe-core on every screen, one
journey at phone size) against
a release build. Assertions read what the player sees (roles, labels, live regions) and what the
game saved (`sixfold.save.<n>` in localStorage).

```bash
cd web && trunk build --release        # the suite serves web/dist
cd ../e2e && npm ci && npx playwright install chromium --only-shell
npx playwright test                    # all journeys, desktop and phone
npx playwright test world --headed     # one spec, in a visible browser
npx playwright show-report             # after a CI failure: download the e2e-report artifact first
```
