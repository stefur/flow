# flow
This is a rewrite of [riverwm-utils](https://github.com/NickHastings/riverwm-utils) in Rust (very cliché, I know).

## Features
- Cycle the focused tags, either to next or previous tag.

## Installation from source
1. Make sure you've got Rust installed. Either via your distributions package manager or [`rustup`](https://rustup.rs/).
2. `cargo install --git https://github.com/stefur/flow flow`

## Usage
Currently the following commands can be used with flow.

| Command | Arguments | Description | Example |
| --- | --- | --- | --- |
| `cycle-tags` | Direction: `next` or `previous`. Number of available tags: `int`, defaults to `9` if omitted. | Move focused tag to the next or previous tag. | `flow cycle-tags next 6` |