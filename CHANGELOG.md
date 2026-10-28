# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/). Entries from 0.1.0 on are written by
release-please from Conventional Commits.

## [1.11.0](https://github.com/phmatray/sixfold/compare/v1.10.0...v1.11.0) (2026-10-28)


### Added

* **desktop:** a desktop app with Tauri, ready for a Steam beta ([#185](https://github.com/phmatray/sixfold/issues/185)) ([#186](https://github.com/phmatray/sixfold/issues/186)) ([ef7040a](https://github.com/phmatray/sixfold/commit/ef7040a05d285c9a23df993e3ee04686cdcced58))
* **duel:** a world duel won with packs offers Open it now ([#208](https://github.com/phmatray/sixfold/issues/208)) ([80a7e04](https://github.com/phmatray/sixfold/commit/80a7e04971ea69816cc97f43c8ff2705fd67fe43))
* **world:** ambient life and micro-events ([#115](https://github.com/phmatray/sixfold/issues/115)) ([#182](https://github.com/phmatray/sixfold/issues/182)) ([890e5d6](https://github.com/phmatray/sixfold/commit/890e5d65f795d10da78d26d523bd428b5c0323a9))
* **world:** Blender-built critter, bird and boat models replace the placeholders ([#189](https://github.com/phmatray/sixfold/issues/189)) ([#203](https://github.com/phmatray/sixfold/issues/203)) ([8c26097](https://github.com/phmatray/sixfold/commit/8c260976aa8669467464357ecca07e2e65d3505f))
* **world:** every inn has an upstairs with its beds ([#193](https://github.com/phmatray/sixfold/issues/193)) ([#199](https://github.com/phmatray/sixfold/issues/199)) ([9adbf24](https://github.com/phmatray/sixfold/commit/9adbf24a0917247347377485b6c06726cd1962a8))
* **world:** NPC routines ([#113](https://github.com/phmatray/sixfold/issues/113)) ([#181](https://github.com/phmatray/sixfold/issues/181)) ([db9228c](https://github.com/phmatray/sixfold/commit/db9228ccf674becd9b09e17e769dfcad3eec3454))
* **world:** open your packs from the world, without going back to the title ([#194](https://github.com/phmatray/sixfold/issues/194)) ([#202](https://github.com/phmatray/sixfold/issues/202)) ([ac33a4e](https://github.com/phmatray/sixfold/commit/ac33a4ed99b90868d5557077bddd3634044406ee))
* **world:** reactions and persistence ([#114](https://github.com/phmatray/sixfold/issues/114)) ([#188](https://github.com/phmatray/sixfold/issues/188)) ([8c7809d](https://github.com/phmatray/sixfold/commit/8c7809dab34ca766aec9d74e7d7aaa71cf9ed579))
* **world:** three camera angles to choose from ([#148](https://github.com/phmatray/sixfold/issues/148)) ([#180](https://github.com/phmatray/sixfold/issues/180)) ([16f5130](https://github.com/phmatray/sixfold/commit/16f51303483363b4499b166179166f8713de6990))


### Fixed

* **cards:** a revealed card no longer shows through its turned-away side in WebKit ([#205](https://github.com/phmatray/sixfold/issues/205)) ([#207](https://github.com/phmatray/sixfold/issues/207)) ([d48f6ed](https://github.com/phmatray/sixfold/commit/d48f6ed7e9223c7a8270ced033a924239cc8c67a))
* **cards:** nothing on a card face spills past its edge ([#195](https://github.com/phmatray/sixfold/issues/195)) ([14b9a68](https://github.com/phmatray/sixfold/commit/14b9a68057ec32be473edda132306aa5f88007e1))
* the 1.11 design audit ([#211](https://github.com/phmatray/sixfold/issues/211)) ([b94a2e2](https://github.com/phmatray/sixfold/commit/b94a2e23eca614c782f0d956f774e71b88ade53d))
* **title:** the way back to the title no longer freezes the page on a software renderer ([#214](https://github.com/phmatray/sixfold/issues/214)) ([4f8cdc4](https://github.com/phmatray/sixfold/commit/4f8cdc4b78279eb34803ffc405521de596b79243))
* **ui:** achievements wear a faceted hex, not a six-pointed star, and are called Succès ([#197](https://github.com/phmatray/sixfold/issues/197)) ([53f253d](https://github.com/phmatray/sixfold/commit/53f253d5ca1894e5f6c691f8a51fce137110f802))
* **ui:** the inn's rest panel reads, and its counter shows its packs and singles ([#196](https://github.com/phmatray/sixfold/issues/196)) ([92f054b](https://github.com/phmatray/sixfold/commit/92f054b7a5e981d6e56da367408b69129a1a7347))
* **ui:** the title's signboards hold still and say their whole line ([#198](https://github.com/phmatray/sixfold/issues/198)) ([2df156a](https://github.com/phmatray/sixfold/commit/2df156af23f80c6778e6a5791a626e7e918eec5d))
* **world:** one owner for the camera angle, and no C/R3 binding clash ([#191](https://github.com/phmatray/sixfold/issues/191)) ([a15ca9b](https://github.com/phmatray/sixfold/commit/a15ca9b06002608972d09900daebd391df5a0aeb))
* **world:** the seal-duel arc's last polish ([#201](https://github.com/phmatray/sixfold/issues/201)) ([186a161](https://github.com/phmatray/sixfold/commit/186a16151569eb435c01033d528260e5eb3911e4))


### Performance

* **world:** hold a 16.7 ms frame with 6 routine walkers ([#187](https://github.com/phmatray/sixfold/issues/187)) ([#200](https://github.com/phmatray/sixfold/issues/200)) ([720fbe7](https://github.com/phmatray/sixfold/commit/720fbe7bb02b4f7e2843623a1f2706da83b2d2cf))


### Changed

* **cards:** Trainer cards become Gear (Objet), and nobody in Sixfold is a trainer ([#192](https://github.com/phmatray/sixfold/issues/192)) ([#204](https://github.com/phmatray/sixfold/issues/204)) ([7af3ed7](https://github.com/phmatray/sixfold/commit/7af3ed702e4a13f402366c9e26cec23767ed55df))


### Documentation

* refresh the README for 1.10, with a banner and screenshots ([#178](https://github.com/phmatray/sixfold/issues/178)) ([4dbcf5a](https://github.com/phmatray/sixfold/commit/4dbcf5a91f42d355dbf1dbfaf72e3bad5ebcf1bf))

## [1.10.0](https://github.com/phmatray/sixfold/compare/v1.9.1...v1.10.0) (2026-10-28)


### Added

* **campaign:** a seal keeper accepts only once you've beaten enough of their region's duelists ([#150](https://github.com/phmatray/sixfold/issues/150)) ([#169](https://github.com/phmatray/sixfold/issues/169)) ([8a4945f](https://github.com/phmatray/sixfold/commit/8a4945f6b3d528396111427623fa4759f9b0dd4d))
* **duel:** a seal duel looks like one ([#151](https://github.com/phmatray/sixfold/issues/151)) ([#168](https://github.com/phmatray/sixfold/issues/168)) ([9524bc6](https://github.com/phmatray/sixfold/commit/9524bc693f3944bea7d1b1e582a209303a1dcf88))
* **tools:** check-maps v2 — blocking and advisory level-design checks ([#110](https://github.com/phmatray/sixfold/issues/110)) ([#164](https://github.com/phmatray/sixfold/issues/164)) ([409958b](https://github.com/phmatray/sixfold/commit/409958bd8847bf688f69b4420ab75a5b361f30aa))
* **tools:** world debug overlays ([#111](https://github.com/phmatray/sixfold/issues/111)) ([#165](https://github.com/phmatray/sixfold/issues/165)) ([238e0a0](https://github.com/phmatray/sixfold/commit/238e0a0c4415b3de322491c5f8dc087ad73fc7c3))
* **world:** an arrival and an entrance at every seal site ([#152](https://github.com/phmatray/sixfold/issues/152)) ([#171](https://github.com/phmatray/sixfold/issues/171)) ([a14f983](https://github.com/phmatray/sixfold/commit/a14f983d57cac55fca3efdd53f2c4964b293b308))
* **world:** Fernhollow Trail, the reference map ([#112](https://github.com/phmatray/sixfold/issues/112)) ([#170](https://github.com/phmatray/sixfold/issues/170)) ([cbaa825](https://github.com/phmatray/sixfold/commit/cbaa82518ae29436bf1be5d65e8cdbda0e2e70f8))
* **world:** the seal is won on stage, and a defeat is met with respect ([#153](https://github.com/phmatray/sixfold/issues/153)) ([#172](https://github.com/phmatray/sixfold/issues/172)) ([7d7548d](https://github.com/phmatray/sixfold/commit/7d7548d4c6cde995c1ae715dcb9c8f73779ed7c9))
* **world:** the six seal keepers stand at their seal sites ([#149](https://github.com/phmatray/sixfold/issues/149)) ([#163](https://github.com/phmatray/sixfold/issues/163)) ([00925d3](https://github.com/phmatray/sixfold/commit/00925d3f779d4265d4145e69c980c9bcb15d964a))
* **world:** the Warden's duel is the grandest ([#154](https://github.com/phmatray/sixfold/issues/154)) ([#173](https://github.com/phmatray/sixfold/issues/173)) ([0242683](https://github.com/phmatray/sixfold/commit/024268321dbf9463d91ce66c64e75eb006d873e1))


### Fixed

* the 1.10 design audit ([#175](https://github.com/phmatray/sixfold/issues/175)) ([bf7930e](https://github.com/phmatray/sixfold/commit/bf7930ebf3d917fd38f713661344c27cfd8e020b))
* **world:** an arrow pressed while a door fades in is no longer lost ([#174](https://github.com/phmatray/sixfold/issues/174)) ([862a27b](https://github.com/phmatray/sixfold/commit/862a27b68be91a980ce24a9fb9ebc6b65e21ca59))
* **world:** the first talk on a map no longer freezes while the speaker's bust is drawn ([#159](https://github.com/phmatray/sixfold/issues/159)) ([#167](https://github.com/phmatray/sixfold/issues/167)) ([da83a28](https://github.com/phmatray/sixfold/commit/da83a285a0f52946da1038e211d08cfb8fc493f5))
* **world:** the seal-duel staging, polished to an independent critique ([#155](https://github.com/phmatray/sixfold/issues/155)) ([#176](https://github.com/phmatray/sixfold/issues/176)) ([7c7c035](https://github.com/phmatray/sixfold/commit/7c7c0351466a2d824bd9b2f06f1b8ad82f227cfc))

## [1.9.1](https://github.com/phmatray/sixfold/compare/v1.9.0...v1.9.1) (2026-10-27)


### Fixed

* **world:** tap to walk is an option on touch screens, the on-screen arrows by default ([#160](https://github.com/phmatray/sixfold/issues/160)) ([47e28d8](https://github.com/phmatray/sixfold/commit/47e28d82da9f4a93c1596e1329042b3673260947))

## [1.9.0](https://github.com/phmatray/sixfold/compare/v1.8.0...v1.9.0) (2026-10-27)


### Added

* a logo for Sixfold ([#128](https://github.com/phmatray/sixfold/issues/128)) ([c4a990f](https://github.com/phmatray/sixfold/commit/c4a990fb4cb4d18c28810c088ec42ab014beb378))
* **a11y:** readable, high-contrast, screen-reader-friendly ([#70](https://github.com/phmatray/sixfold/issues/70)) ([#94](https://github.com/phmatray/sixfold/issues/94)) ([4d38132](https://github.com/phmatray/sixfold/commit/4d3813284e64b59138c444fff8c3d207effa04d6))
* **decks:** a deck builder with an auto mode, suggestions and unlockable decks ([#102](https://github.com/phmatray/sixfold/issues/102)) ([#103](https://github.com/phmatray/sixfold/issues/103)) ([3cf7484](https://github.com/phmatray/sixfold/commit/3cf7484eb144ec1d5c0631202e02f2e641cead3b))
* **ui:** a title that shows the world, and game-like menus ([#101](https://github.com/phmatray/sixfold/issues/101)) ([ac568a4](https://github.com/phmatray/sixfold/commit/ac568a48ecf59ddcd404c5dd4562a158ee6fc356))
* **ui:** screen transitions and one visual grammar ([#60](https://github.com/phmatray/sixfold/issues/60)) ([#96](https://github.com/phmatray/sixfold/issues/96)) ([ba2f169](https://github.com/phmatray/sixfold/commit/ba2f16947d3ed9a203c4703aa472eca512926616))
* **world:** a challenge panel that stays in the world ([#108](https://github.com/phmatray/sixfold/issues/108)) ([93e32bb](https://github.com/phmatray/sixfold/commit/93e32bbad7a032a1941fa27e4de34ec14e499864))
* **world:** books you can take from the shelves to read about the rules and the world ([#104](https://github.com/phmatray/sixfold/issues/104)) ([#106](https://github.com/phmatray/sixfold/issues/106)) ([1e9e4a6](https://github.com/phmatray/sixfold/commit/1e9e4a67314a65e13153c2579a31b84a4ee00750))
* **world:** doors that open on every building, and model fixes ([#126](https://github.com/phmatray/sixfold/issues/126)) ([e2db937](https://github.com/phmatray/sixfold/commit/e2db9371f934922b921a1b60562295b3bd80dbc3))
* **world:** sit on benches and chairs ([#107](https://github.com/phmatray/sixfold/issues/107)) ([275552d](https://github.com/phmatray/sixfold/commit/275552df390ca559b0a14e12144e2d58cc9a3da3))


### Fixed

* 1.9 design audit and stable browser tests ([#138](https://github.com/phmatray/sixfold/issues/138)) ([9503417](https://github.com/phmatray/sixfold/commit/950341740701565c0db6836896f59c397e91f7ae))
* **world:** the hero climbs porch steps instead of walking through them ([#109](https://github.com/phmatray/sixfold/issues/109)) ([6fa7931](https://github.com/phmatray/sixfold/commit/6fa7931e368511345f9c38f2fd8e95cbb6df3a21))


### Changed

* run the web crate's tests ([#156](https://github.com/phmatray/sixfold/issues/156)) ([105401d](https://github.com/phmatray/sixfold/commit/105401d25d3a9d40569ae841c08dda78513b4fb5))


### Documentation

* Deep Seals moves to 1.17, before 2.0 ([#139](https://github.com/phmatray/sixfold/issues/139)) ([e444003](https://github.com/phmatray/sixfold/commit/e44400389be5155744c58891b017d75b6bcd0398))
* level-design guide and the world roadmap (1.10 to 1.14) ([#125](https://github.com/phmatray/sixfold/issues/125)) ([94ea881](https://github.com/phmatray/sixfold/commit/94ea881b1b225da2126182fccc372ea534271a29))
* refresh the roadmap (1.9 to 2.0) ([#157](https://github.com/phmatray/sixfold/issues/157)) ([5b9290a](https://github.com/phmatray/sixfold/commit/5b9290a539a61a56bfb66c786567054c8d907c60))
* set design analysis and two expansions ([#127](https://github.com/phmatray/sixfold/issues/127)) ([349f50c](https://github.com/phmatray/sixfold/commit/349f50cb08cf419a17e2211b7fd18eba3e595072))

## [1.8.0](https://github.com/phmatray/sixfold/compare/v1.7.0...v1.8.0) (2026-10-26)


### Added

* a final victory sequence and staged credits ([#67](https://github.com/phmatray/sixfold/issues/67)) ([#93](https://github.com/phmatray/sixfold/issues/93)) ([21e59cc](https://github.com/phmatray/sixfold/commit/21e59cc0fb6bc131cccbeb099151bf89e1322f49))
* **duel:** juice — hit-stop, damage stamps, prize flight and feedback sounds ([#61](https://github.com/phmatray/sixfold/issues/61)) ([#92](https://github.com/phmatray/sixfold/issues/92)) ([a117dcc](https://github.com/phmatray/sixfold/commit/a117dcc161b76ca2c75f6b6843cf88ea6c196c0f))


### Fixed

* 1.8 design audit — duel juice, finale and credits ([#99](https://github.com/phmatray/sixfold/issues/99)) ([be715de](https://github.com/phmatray/sixfold/commit/be715de1743df5b52765c2a28a896a84b8181d75))

## [1.7.0](https://github.com/phmatray/sixfold/compare/v1.6.0...v1.7.0) (2026-10-26)


### Added

* **cards:** a binder worth completing and better pack openings ([#68](https://github.com/phmatray/sixfold/issues/68)) ([#80](https://github.com/phmatray/sixfold/issues/80)) ([7584944](https://github.com/phmatray/sixfold/commit/758494462490df62b8fc3d7713e8fa30b18b9799))
* gamepad support ([#69](https://github.com/phmatray/sixfold/issues/69)) ([#86](https://github.com/phmatray/sixfold/issues/86)) ([932a5fb](https://github.com/phmatray/sixfold/commit/932a5fbb79ca6e4b3e6d6fd49f346dc8cf82c38e))
* **story:** Rook and the seventh face ([#64](https://github.com/phmatray/sixfold/issues/64)) ([#90](https://github.com/phmatray/sixfold/issues/90)) ([e92bd61](https://github.com/phmatray/sixfold/commit/e92bd61162b9d63bd2d0ad422ab899f8abee50ae))


### Fixed

* 1.7 design audit — Rook's story, binder and packs, gamepad prompts ([#97](https://github.com/phmatray/sixfold/issues/97)) ([7d20b72](https://github.com/phmatray/sixfold/commit/7d20b72ac95a6faa9f3700a4721faa0ab2fe2c26))

## [1.6.0](https://github.com/phmatray/sixfold/compare/v1.5.0...v1.6.0) (2026-10-26)


### Added

* **duel:** a redesigned duel screen ([#48](https://github.com/phmatray/sixfold/issues/48)) ([55f86ee](https://github.com/phmatray/sixfold/commit/55f86eecfba039b6ed5fee4d2327a9f6a37841bc))
* **title:** a studio splash and a living title screen ([#66](https://github.com/phmatray/sixfold/issues/66)) ([#82](https://github.com/phmatray/sixfold/issues/82)) ([c6626df](https://github.com/phmatray/sixfold/commit/c6626dfa7fe3c8c3e26907c6c26dd09cb7628b2d))
* **world:** cutscene scripts ([#63](https://github.com/phmatray/sixfold/issues/63)) ([#85](https://github.com/phmatray/sixfold/issues/85)) ([d714307](https://github.com/phmatray/sixfold/commit/d714307641a2b4e640c7c90eeed1d6f0dfa40f9e))


### Fixed

* 1.6 design audit and loose ends ([#55](https://github.com/phmatray/sixfold/issues/55)) ([#89](https://github.com/phmatray/sixfold/issues/89)) ([e79ec11](https://github.com/phmatray/sixfold/commit/e79ec11e810d0580cd307989275dad6233a8cb63))

## [1.5.0](https://github.com/phmatray/sixfold/compare/v1.4.0...v1.5.0) (2026-10-26)


### Added

* **audio:** a soundscape per region and music for rivals and bosses ([#71](https://github.com/phmatray/sixfold/issues/71)) ([#81](https://github.com/phmatray/sixfold/issues/81)) ([316f1ec](https://github.com/phmatray/sixfold/commit/316f1ec2253308db42c4348daee7ed9e92e911d7))
* stats and achievements ([#65](https://github.com/phmatray/sixfold/issues/65)) ([#79](https://github.com/phmatray/sixfold/issues/79)) ([6086ad9](https://github.com/phmatray/sixfold/commit/6086ad935508ae2284a00737c614e8645ae49e69))
* **world:** dialogue polish, current objective and clear unlocks ([#62](https://github.com/phmatray/sixfold/issues/62)) ([#78](https://github.com/phmatray/sixfold/issues/78)) ([d37b494](https://github.com/phmatray/sixfold/commit/d37b4940849447a22660d58fda623f98fad3acf6))


### Fixed

* **ui:** 1.5 design audit — dialogue, objective, records, audio settings ([#87](https://github.com/phmatray/sixfold/issues/87)) ([0577c48](https://github.com/phmatray/sixfold/commit/0577c486bcc95db1c19385d56b8a879b2f60c78c))

## [1.4.0](https://github.com/phmatray/sixfold/compare/v1.3.0...v1.4.0) (2026-10-26)


### Added

* play fully offline from a fresh install, with a loading screen ([#57](https://github.com/phmatray/sixfold/issues/57)) ([#77](https://github.com/phmatray/sixfold/issues/77)) ([7c6acdc](https://github.com/phmatray/sixfold/commit/7c6acdc02808efe0eb8d4eef2339e585a2643fe0))
* **saves:** survive corruption and test every migration ([#58](https://github.com/phmatray/sixfold/issues/58)) ([#76](https://github.com/phmatray/sixfold/issues/76)) ([7fb140b](https://github.com/phmatray/sixfold/commit/7fb140bd7c06f7f87329df32b9c7f5e9765a6c79))
* **ui:** an in-game menu ([#56](https://github.com/phmatray/sixfold/issues/56)) ([#75](https://github.com/phmatray/sixfold/issues/75)) ([35190eb](https://github.com/phmatray/sixfold/commit/35190eb79f740ee0a2fe56870f8be003e6ca48ec))


### Fixed

* **ui:** 1.4 design audit — in-game menu, save recovery, loading ([#83](https://github.com/phmatray/sixfold/issues/83)) ([3ebda7c](https://github.com/phmatray/sixfold/commit/3ebda7ca20fa7535a527402748354c32d92750f4))


### Documentation

* roadmap 1.4 to 2.0 and a release every 3 PRs ([#73](https://github.com/phmatray/sixfold/issues/73)) ([a04270d](https://github.com/phmatray/sixfold/commit/a04270d5a8e87d94be29d5d88b410b846cbb5aa4))

## [1.3.0](https://github.com/phmatray/sixfold/compare/v1.2.0...v1.3.0) (2026-10-25)


### Added

* **balance:** first-turn rule, one retreat a turn, value budget and card pass ([#45](https://github.com/phmatray/sixfold/issues/45)) ([5762327](https://github.com/phmatray/sixfold/commit/5762327f263aa223f2a96fd7dd39743c3e790e8d))
* call creatures Hexons and evolution Awakening ([#38](https://github.com/phmatray/sixfold/issues/38)) ([12c5249](https://github.com/phmatray/sixfold/commit/12c5249be7f6bbe429a398ade8d6c33071697e31))
* **cards:** Hexon lore and redesigned illustrations ([#43](https://github.com/phmatray/sixfold/issues/43)) ([2e1fd50](https://github.com/phmatray/sixfold/commit/2e1fd501f3ba6d877dc11df8beee1bef08ee10c9))
* **content:** Ember and Storm ([#37](https://github.com/phmatray/sixfold/issues/37)) ([e51a373](https://github.com/phmatray/sixfold/commit/e51a373ad1feb9e792e60932184b60318e1aaaaa))
* **content:** Grove and Stone ([#53](https://github.com/phmatray/sixfold/issues/53)) ([08c51e4](https://github.com/phmatray/sixfold/commit/08c51e40d41c489b5b192ef091b58fdedb35d681))
* **content:** Tide and Dream ([#40](https://github.com/phmatray/sixfold/issues/40)) ([1b5d2e5](https://github.com/phmatray/sixfold/commit/1b5d2e55872f38d4ecf35c7e9ba7d3b52b7c634a))
* name the seven types after the faces of the world ([#52](https://github.com/phmatray/sixfold/issues/52)) ([ee092c2](https://github.com/phmatray/sixfold/commit/ee092c2e102d4f6e2b81c54f0e59e36259b2aad4))
* **packs:** designed boosters, slot odds, holo foil and pack opening ([#39](https://github.com/phmatray/sixfold/issues/39)) ([458c816](https://github.com/phmatray/sixfold/commit/458c81632ef723df6de5f604018554521b38c5ff))
* **world:** a lift you can ride, on a proper scaffold ([#51](https://github.com/phmatray/sixfold/issues/51)) ([a7d539d](https://github.com/phmatray/sixfold/commit/a7d539d177e4b31ccaba005351eab054bb35cc42))
* **world:** day and night, gates, world map and fast travel ([#34](https://github.com/phmatray/sixfold/issues/34)) ([fbdeb61](https://github.com/phmatray/sixfold/commit/fbdeb61abb438536786b3cf7f3edb183fcba783c))
* **world:** fireflies mark finds instead of a blinking light ([#49](https://github.com/phmatray/sixfold/issues/49)) ([1dd6bc9](https://github.com/phmatray/sixfold/commit/1dd6bc961a5b7dd368a62d00221c262a83b38584))
* **world:** glowing mushrooms light caves, never street lamps ([#42](https://github.com/phmatray/sixfold/issues/42)) ([c01c8c1](https://github.com/phmatray/sixfold/commit/c01c8c1594931b89f758fbb9b8c71f90c3de372a))
* **world:** grass clumps instead of crossed planes ([#50](https://github.com/phmatray/sixfold/issues/50)) ([290d9e8](https://github.com/phmatray/sixfold/commit/290d9e8f478a277c18401c1acaa49fc7c52a45bc))
* **world:** organic terrain instead of block cliffs ([#44](https://github.com/phmatray/sixfold/issues/44)) ([f7e64c3](https://github.com/phmatray/sixfold/commit/f7e64c387ea2d20b29fcf71052d1f795bfd3642d))
* **world:** painted-figurine hero, NPCs and rivals ([#46](https://github.com/phmatray/sixfold/issues/46)) ([759e3e4](https://github.com/phmatray/sixfold/commit/759e3e43e0268fddc06e62b8c1f6622f7ab54bf8))
* **world:** placement rules and checks ([#41](https://github.com/phmatray/sixfold/issues/41)) ([fa73a59](https://github.com/phmatray/sixfold/commit/fa73a59d124b0a63d94f8f886c6805e017fc1b54))


### Fixed

* **cards:** reinvent the Colorless Hexons ([#47](https://github.com/phmatray/sixfold/issues/47)) ([72d0f9b](https://github.com/phmatray/sixfold/commit/72d0f9b60306d9dcc88d4a4b36a789bdfc52e011))
* **cards:** show the whole illustration on full cards ([#36](https://github.com/phmatray/sixfold/issues/36)) ([353eb9e](https://github.com/phmatray/sixfold/commit/353eb9e63e6d9af5aa17c2956344cfb23af18a43))
* **world:** re-lay every map to the world rules ([#54](https://github.com/phmatray/sixfold/issues/54)) ([9a81e22](https://github.com/phmatray/sixfold/commit/9a81e22ed50019b2795af587e6e653bcd3cb6c0c))

## [1.2.0](https://github.com/phmatray/sixfold/compare/v1.1.0...v1.2.0) (2026-10-25)


### Added

* **content:** duelists, finds, quests and six houses ([#32](https://github.com/phmatray/sixfold/issues/32)) ([1dd51f9](https://github.com/phmatray/sixfold/commit/1dd51f9bef72c0c0b0ef71450d7df3ac34b62a9a))
* **engine:** facets, shops, route duelists and rival styles ([#28](https://github.com/phmatray/sixfold/issues/28)) ([be82f18](https://github.com/phmatray/sixfold/commit/be82f18e0d6e0a0bb1791973e84f5d1dc7f73985))
* **world:** line of sight, finds and houses ([#31](https://github.com/phmatray/sixfold/issues/31)) ([78c8b70](https://github.com/phmatray/sixfold/commit/78c8b70cf5428ef586f3f11d0fc747895d3e90d1))


### Fixed

* **ui:** 1.2 design audit — phone board fit, full inspector, interior camera, clinic look, French pass ([#33](https://github.com/phmatray/sixfold/issues/33)) ([8739fdc](https://github.com/phmatray/sixfold/commit/8739fdc9dff280872c79f158690a71df8ddf1b56))


### Changed

* **world:** split world.js into modules ([#29](https://github.com/phmatray/sixfold/issues/29)) ([3867cd3](https://github.com/phmatray/sixfold/commit/3867cd34cef50c4c3f075e47f18af6ac46a56c20))

## [1.1.0](https://github.com/phmatray/sixfold/compare/v1.0.0...v1.1.0) (2026-10-25)


### Added

* **cards:** a real card face with illustrations, attacks and foil ([#25](https://github.com/phmatray/sixfold/issues/25)) ([2ee8b9b](https://github.com/phmatray/sixfold/commit/2ee8b9b55e5c1bb7fcfc6abebd255c0f80805383))
* **cards:** painted illustrations for all 110 cards ([#26](https://github.com/phmatray/sixfold/issues/26)) ([7b2912a](https://github.com/phmatray/sixfold/commit/7b2912a5e5c8bd861e82a71d9be18c37947fe320))
* **i18n:** play in French or English ([#23](https://github.com/phmatray/sixfold/issues/23)) ([f96e214](https://github.com/phmatray/sixfold/commit/f96e2145620946653e2f5f30969f13d9a68a873a))
* **tutorial:** a guided first duel and a Continue button ([#24](https://github.com/phmatray/sixfold/issues/24)) ([b37986b](https://github.com/phmatray/sixfold/commit/b37986be690845f61511aed28ecc6b7c43d2cc98))


### Fixed

* **mobile:** no page zoom or text selection, tap to interact, calmer running ([#21](https://github.com/phmatray/sixfold/issues/21)) ([cccd47f](https://github.com/phmatray/sixfold/commit/cccd47f24091a3bfad6b1f950f0861ee76a37e8f))
* **ui:** 1.1 design audit — phone board fit, full hand, retreat lesson, atomic world cache ([#27](https://github.com/phmatray/sixfold/issues/27)) ([e3f52e1](https://github.com/phmatray/sixfold/commit/e3f52e1ff7f51df56a6887d0856e18e12096476c))

## [1.0.0](https://github.com/phmatray/sixfold/compare/v0.9.0...v1.0.0) (2026-10-24)


### Added

* final polish for the 1.0 release ([#18](https://github.com/phmatray/sixfold/issues/18)) ([2c7eac5](https://github.com/phmatray/sixfold/commit/2c7eac5afe9700f1a46ce878aa1cc78093bdb11d))

## [0.9.0](https://github.com/phmatray/sixfold/compare/v0.8.0...v0.9.0) (2026-10-24)


### Added

* **endgame:** credits, a post-game Gauntlet, daily duels, promo cards and a balance pass ([#16](https://github.com/phmatray/sixfold/issues/16)) ([55e95e1](https://github.com/phmatray/sixfold/commit/55e95e10bc39eeb23d7662092b0fb09e18465c68))

## [0.8.0](https://github.com/phmatray/sixfold/compare/v0.7.0...v0.8.0) (2026-10-24)


### Added

* **polish:** save slots, offline play, a Hard AI and an accessibility pass ([#14](https://github.com/phmatray/sixfold/issues/14)) ([f8285c2](https://github.com/phmatray/sixfold/commit/f8285c2c47f0961d5dfe1a8536ccc375549b2ce5))

## [0.7.0](https://github.com/phmatray/sixfold/compare/v0.6.0...v0.7.0) (2026-10-24)


### Added

* **world:** open four themed areas around the meadow with side quests ([#12](https://github.com/phmatray/sixfold/issues/12)) ([850d5e8](https://github.com/phmatray/sixfold/commit/850d5e8eb7b995e063931f905bb493b35a2527a6))

## [0.6.0](https://github.com/phmatray/sixfold/compare/v0.5.0...v0.6.0) (2026-10-24)


### Added

* **campaign:** duel six rivals and the Hex Warden from the world for seals and packs ([#10](https://github.com/phmatray/sixfold/issues/10)) ([b9bc313](https://github.com/phmatray/sixfold/commit/b9bc313a847285e406a953cf306d137ca59a12dd))

## [0.5.0](https://github.com/phmatray/sixfold/compare/v0.4.0...v0.5.0) (2026-10-24)


### Added

* **audio:** add original synthesised music and sound effects ([#8](https://github.com/phmatray/sixfold/issues/8)) ([cbaeee1](https://github.com/phmatray/sixfold/commit/cbaeee14f0e8d03fb898972e02e584055f07d306))

## [0.4.0](https://github.com/phmatray/sixfold/compare/v0.3.0...v0.4.0) (2026-10-24)


### Added

* **cards:** grow the set to 103 original cards with four new decks and a starter ([#6](https://github.com/phmatray/sixfold/issues/6)) ([17c4313](https://github.com/phmatray/sixfold/commit/17c43130d667db61c3384727fc920157f5e028e2))

## [0.3.0](https://github.com/phmatray/sixfold/compare/v0.2.0...v0.3.0) (2026-10-24)


### Added

* collect cards, open booster packs and build your own decks ([#4](https://github.com/phmatray/sixfold/issues/4)) ([ac39bf9](https://github.com/phmatray/sixfold/commit/ac39bf99319a8273e516349f9e48ae1621f617bc))

## [0.2.0](https://github.com/phmatray/sixfold/compare/v0.1.0...v0.2.0) (2026-10-24)


### Added

* **world:** explore a 3D overworld with an enterable duel hall, NPCs and running shoes ([#2](https://github.com/phmatray/sixfold/issues/2)) ([b7782af](https://github.com/phmatray/sixfold/commit/b7782afd0edc888b570aa906c03966218fb5c99d))

## [0.1.0](https://github.com/phmatray/sixfold/releases/tag/v0.1.0) (2026-10-24)

### Added

- Rules engine in pure Rust: setup and mulligans, turn structure, energy, retreat, special
  conditions, damage with weakness and resistance, evolution, knock-outs, prizes and sudden death,
  trainers, powers and the effect framework, deck validation. Every rule of `docs/RULES.md` is
  covered by a named unit test (241 tests).
- An original card set and two balanced prebuilt decks.
- An AI opponent (Easy and Normal).
- A Leptos/WebAssembly match UI: board, hand, interactive turn, conditions and prizes, coin flips,
  battle log, card inspector, animations; desktop and phone layouts.
