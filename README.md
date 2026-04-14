# rustrix

A cinematic Matrix-style terminal rain written in Rust.

## Install

```
cargo install rustrix
```

Or build from source:

```
git clone https://github.com/deatherving/rustrix
cd rustrix
cargo build --release
./target/release/rustrix
```

## Usage

```
rustrix [OPTIONS]
```

| Option | Description |
|---|---|
| `-l`, `--letters` | Letters mode — A–Z, a–z, 0–9 (default) |
| `-b`, `--binary` | Binary mode — 0 and 1 only |

## Controls

| Key | Action |
|---|---|
| `q` / `Esc` / `Enter` | Quit |
| `Ctrl+C` | Quit |

## Font

The visual appearance depends on your terminal's font. For the closest match to the original Matrix film, set your terminal font to **Matrix Code NFI** (available as a free download).

## License

MIT
