# Rus

A modern Minecraft: Java Edition launcher written in Rust. Rus is its own environment, not a page in your browser. Play starts the client inside a sandbox.

English interface. Black, sharp, and quiet. Sidebar on the left. Type is [Mozilla Headline](https://github.com/mozilla/mozilla-type-family) (SIL Open Font License).

Rus is not affiliated with Mojang or Microsoft. You need your own copy of Minecraft, which Rus downloads from Mojang’s servers into your data folder.

## Download

| Platform | File |
| --- | --- |
| Linux x86-64 | `rus-linux-x86_64` |
| Windows x64 | `rus-windows-x64.exe` |

Linux:

```bash
chmod +x rus-linux-x86_64
./rus-linux-x86_64
```

Windows: run `rus-windows-x64.exe`. Rus does not open a browser.

The binary is the environment. Play starts Minecraft inside the sandbox, not in a browser tab. On Linux, install `bubblewrap` for the kernel jail (`apt install bubblewrap`). Without it, the client still gets a private profile under the data folder. On Windows the client is placed in a job object with that same private profile.

| Flag | |
| --- | --- |
| `--port <n>` | Local port. Default `8899`. `0` picks a free port. |
| `--dir <path>` | Data folder. Default `~/.rus` on Linux, `%APPDATA%\Rus` on Windows. |

## What it does

Four pages: **Home**, **Versions**, **Mods**, **Settings**.

- Local account only. The offline UUID matches vanilla (`OfflinePlayer:<name>`).
- Releases are listed by default. Snapshots and previews from 26 upward appear only after you turn them on in Settings.
- Default skin is Steve. One bundled cape, **Rus**, in the real 64×32 cape layout. You can upload your own PNG skin or cape.
- Install is a set of patches you can turn off before it starts: vanilla, assets, Java, skin, mods. Progress is live and cancelable.
- Large downloads are split into 2 MB parts. A dropped connection or a bad part retries that part only. Finished parts stay on disk, so the next attempt continues instead of starting over.
- Next to Play: **Vanilla** (the official Java client) or **Horizon** (the Rust rewrite of 26.3). Horizon uses your installed 26.3 files. It does not replace them. [Horizon](https://github.com/ludovic111/horizon) is an unofficial project.
- Play does not open a browser. On Linux with bubblewrap, the client runs in its own mount, pid and hostname namespaces (`rus-sandbox`). Your home directory is not mounted, and the process dies when Rus exits. On Windows it runs in a job object with a private profile under Rus.

## Build

Rust 1.80 or newer.

```bash
cargo build --release
./target/release/rus
```

Windows from Linux needs the MinGW linker and the `x86_64-pc-windows-gnu` target. `.cargo/config.toml` points the linker at `x86_64-w64-mingw32-gcc`.

```bash
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
```

## Layout

```
src/main.rs      entry
src/server.rs    local HTTP API and the embedded UI
src/mojang.rs    version manifest, releases plus optional 26+ snapshots
src/install.rs   patch pipeline
src/net.rs       chunked, resumable downloads
src/java.rs      Temurin JRE when the system JVM is too old
src/skin.rs      offline skin pack and the Rus cape
src/mods.rs      mod library
src/play.rs      Java command line
src/sandbox.rs   kernel jail (Linux) or job object (Windows)
src/horizon.rs   optional Horizon 26.3 client
ui/              English interface, embedded in the binary
assets/          Mozilla Headline, Steve, Alex
```
