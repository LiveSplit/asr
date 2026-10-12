# Harness

Runs the Unity players from [auto-splitting-test-fixtures](https://github.com/LiveSplit/auto-splitting-test-fixtures) against an auto splitter built on asr and checks what the auto splitter reads against `unity-fixture.json`, the file that says what every player holds.

```
cargo run --release
```

The first run fetches the players for this machine's platform from the fixtures releases, checks each zip against the manifest and unpacks it under `target/fixtures`. Set `ASR_FIXTURES` to keep them somewhere else. Every run after that starts from the cache. The players are pinned to one commit of the fixtures repo, `FIXTURES` in `src/main.rs`, so a new corpus is a bump of that constant.

Each player starts headless with `-batchmode -nographics`. The auto splitter in `splitter/` attaches to it, reads the scenes, the objects, and the `FixtureData` statics and fields, and reports every value through `timer::set_variable` under its path in the contract, such as `scenes.0.roots.0.name`. The run prints one line per player and, for a failing one, what was wrong and what the auto splitter said. A player fails when a reported value differs from the contract, when a required path is missing, or when the auto splitter has not said it is done after 90 seconds.

An argument narrows the run to the players whose version or variant contains it:

```
cargo run --release -- 6000.5.10f1
cargo run --release -- win-x86-il2cpp
```
