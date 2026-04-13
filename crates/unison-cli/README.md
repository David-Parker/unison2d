# unison-cli

One-stop command-line tool for creating, building, and running [Unison 2D](https://github.com/David-Parker/unison2d) game projects. Targets Web, iOS, and Android from a single codebase with Lua or TypeScript scripting.

## Install

    cargo install unison-cli

## Quickstart

    unison new my-game
    cd my-game
    unison doctor      # check your toolchain
    unison dev web     # run locally with hot reload

## Commands

| Command | Purpose |
|---------|---------|
| `unison new <name>` | Scaffold a new project (Web + iOS + Android by default) |
| `unison dev <platform>` | Run the dev loop (web: trunk serve; ios/android: prints IDE hints) |
| `unison build <platform>` | Build for `web`, `ios`, `android`, or `all` |
| `unison test` | Run `cargo test` plus `npm test` (TS projects) |
| `unison clean` | Remove build artifacts |
| `unison doctor` | Report missing toolchain pieces |
| `unison platform add/remove <name>` | Add or remove a platform from an existing project |
| `unison link <path>` / `unison unlink` | Point the project at a local engine checkout |

Each subcommand has detailed `--help` text. Try `unison new --help`.

## Versioning

The CLI and the engine ship together. CLI version `X.Y.Z` scaffolds projects pinned to engine `X.Y.Z`. Use `--engine-tag` on `unison new` to override (engine-dev only).

## Link Mode

If you are iterating on the engine alongside a game project, use `unison link`:

    # In the project directory:
    unison link ../unison2d

This adds a `[patch]` entry to the project's `Cargo.toml` pointing at your local engine checkout. Undo with `unison unlink`.

## Language Choice

    unison new my-game --lang ts    # TypeScript (transpiled to Lua via TSTL)
    unison new my-game --lang lua   # Lua (default)

Both produce identical platform shells; only the `project/scripts-src/` layout differs.

## Platform Flags

Skip platforms at project creation time:

    unison new my-game --no-ios --no-android   # web only

Add a platform later:

    unison platform add android

Remove a platform:

    unison platform remove ios

## License

MIT
