# flow
A small utility that brings some extra commands to control [river](https://github.com/riverwm/river).  
Originally inspired by [riverwm-utils](https://github.com/NickHastings/riverwm-utils).

## Features
Currently the following commands can be used with flow.

| Command | Arguments | Description | Example |
| --- | --- | --- | --- |
| `cycle-tags` | Direction: `next` or `previous`. Number of available tags: `int`, defaults to `9` if omitted. | Move focused tag to the next or previous tag. | `flow cycle-tags next 6` |
| `toggle-tags` | Tags to focus. | Focus tags or toggle previous tags if already focused. | `flow toggle-tags 64` |
| `focus-urgent-tags` | None. | Focus urgent tags on an output. | `flow focus-urgent-tags` |
| `focus-set-view-tags` | Tags to set and focus. | Set tags for a view and then focus the tags | `flow focus-set-view-tags 16` |

## Installation from source
1. Make sure you've got Rust installed. Either via your distributions package manager or [`rustup`](https://rustup.rs/).
2. `cargo install --git https://github.com/stefur/flow flow`