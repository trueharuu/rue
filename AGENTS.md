# AGENTS.md

Rue is a TETR.IO Tetris AI bot — beam search over a custom evaluator, connecting via WebSocket to play competitively.

## Nightly Required

All crates use `edition = "2024"` and `#![feature(portable_simd, min_adt_const_params, const_trait_impl)]`. You **must** use a nightly toolchain.

## Commands

| Task | Command |
|------|---------|
| Build | `cargo build --release --workspace` |
| Test | `cargo test --release --workspace` |
| Lint | `cargo clippy --workspace --all-targets` |
| Format | `cargo fmt` (check: `cargo fmt --check`) |
| Bench | `cargo bench --package rue_search` |
| Solo (offline play) | `cargo run --release --package rue_solo -- --load weights/simple.json -n 500` |
| Perft (move-gen perf) | `cargo run --release --package rue_perft` |

There is no CI. Run clippy + fmt + test before committing. For any change that occurs in movegen (`rue_nav::movegen`) should be personally tested with `rue_perft`:
```sh
cargo run --release -- IOLJSZT
```
Assert that this value is equal to exactly **2647076135**.

## Workspace Layout

```
rue_core    — foundation: bitboard, pieces, game state, rulesets (zero deps)
rue_nav     — move generation, collision, pathfinding (pathfinder is a stub)
rue_eval    — evaluation functions: linear Simple + CNN Deep (candle)
rue_search  — beam search (rayon, futex, transposition table)
rue_macro   — proc-macro: #[command] for chat command definitions
rue_client  — TETR.IO WebSocket bot (tokio, triangle-rs) ← main product
rue_solo    — offline singleplayer CLI (clap)
rue_perft   — move-gen performance counter
rue_tuner   — SPSA weight tuner (main is empty, infra exists)
ui/         — placeholder (empty TypeScript)
weights/    — saved weight JSON files
ref/        — reference bot implementations (not part of workspace)
```

**Dependency flow**: `rue_core` → `rue_nav` / `rue_eval` → `rue_search` → `rue_client` / `rue_solo` / `rue_tuner`

## Architecture Notes

- **Board**: `Board<N>` — packed bitboard using `Simd<u64, N>`. Each `u64` band stores 6 rows × 10 cols. Line clears use a constant-time bit trick.
- **Move**: `Move` is a compact 32-bit packed integer (piece | rotation | x | y | spin).
- **Const generics everywhere**: `Piece` and `Spins` derive `ConstParamTy` for monomorphized move-gen (`generate_inlined::<Piece::T, Spins::AllMini, 8>()`).
- **Beam search defaults**: depth 14, width 800. Bot uses depth 7, width 300.
- **Evaluator**: `Simple` (38-parameter linear) loads from `weights/simple.json`. `Deep` (CNN via candle) loads from safetensors.
- **pathfinder is a stub**: `get_input()` returns empty `SmallVec`. Actual movement is handled by `triangle-rs`.

## Lint Config

Clippy is strict: `pedantic` warn, `perf` **deny**, `missing_docs` warn (rustc), `missing_docs_in_private_items` warn. Several lint allows are configured in workspace `Cargo.toml` — check before relaxing.

## Style

- `rustfmt.toml`: `imports_granularity = "Item"` (merges use statements).
- The `.env` file (gitignored) holds `TOKEN`, `PREFIX`, `HOSTS`, `DEV_ROOM_ID` for the TETR.IO client. Copy `.env.example`.
- `run.txt` is a fumen replay output file (gitignored, overwritten by `rue_solo`).
- `ref/` contains reference implementations — read-only algorithmic references, not built by cargo.
