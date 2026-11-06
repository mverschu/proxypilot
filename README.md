# proxypilot

Wrapper around **proxychains** (see below for the binary name): reads local SOCKS proxy ports from a config file, optionally lets you pick one in a small inline TUI (same terminal scrollback, not a fullscreen alternate screen), writes a **temporary** proxychains configuration, and runs:

```text
proxychains4 -f <temporary-config> <your-command> [args...]
```

Proxypilot needs **proxychains-ng** (the `proxychains4` program, sometimes also named `proxychains`). It looks for **`proxychains4` on `PATH` first**, then **`proxychains`** only if that file is **not** the old **ProxyChains 3.x** shell script (that version has no `-f` and causes errors like `exec: -f: not found`). Install the **`proxychains` / `proxychains-ng`** package so `proxychains4` exists.

Override which binary to run:

```bash
export PROXYCHAINS_BIN=/usr/bin/proxychains4
```

All arguments after `proxypilot` are forwarded unchanged. The child process’s exit code is returned. The temporary file is removed after the command finishes; nothing under `/etc/proxychains.conf` is modified.

By default, **proxychains-ng bootstrap lines** on stderr (`[proxychains] config file found`, `preloading`, `DLL init`, …) are **hidden** so only your tool’s output shows. To show them again (debugging):

```bash
export PROXYPILOT_SHOW_PROXYCHAINS_DEBUG=1
```

## Config file

Default path: `$XDG_CONFIG_HOME/proxypilot/ports`, or `~/.config/proxypilot/ports` if `XDG_CONFIG_HOME` is unset.

Override with:

```bash
export PROXYPILOT_CONFIG=/path/to/ports
```

Create or overwrite that file interactively:

```bash
proxypilot init
```

You are prompted for each SOCKS port (127.0.0.1 is always used) and an optional label; finish with a blank port line. If the file already exists, you are asked before overwriting.

File format: one route per line (empty lines and `#` comments allowed):

```text
1080
1081
OfficeNetwork:1080
[Home VPN]:1081
```

- **Port only** (`1080`) — no label; the picker shows `127.0.0.1:1080`.
- **Label and port** (`Name:1080` or `[Name]:1080`) — optional human-readable name for the TUI only; proxychains still uses `127.0.0.1` and that port. Use brackets if the label contains `:`.
- **One route** → that port is used automatically (label is ignored unless you add more lines later).
- **Several routes** → arrow keys (↑/↓ or `j`/`k`) to move, **Enter** to confirm, **Esc** to cancel. The list is **colorized** on ANSI terminals (cyan header, green caret on the selected row).

## Build

```bash
cargo build --release
```

Install the binary from `target/release/proxypilot` into your `PATH`.

## Precompiled binary (Linux)

Prebuilt **Linux x86_64** binaries are attached to [GitHub Releases](https://github.com/mverschu/proxypilot/releases) (look for `proxypilot-linux-x86_64` and `proxypilot-linux-x86_64.tar.gz`). They are dynamically linked against **glibc** (typical on mainstream distros). Copy the binary somewhere on your `PATH` (for example `~/.local/bin/proxypilot`) and ensure it is executable (`chmod +x`). For other targets, build from source with `cargo build --release`.

## Usage

```bash
proxypilot init
proxypilot nxc smb 192.168.1.10
proxypilot nxc smb 192.168.1.10 -u alice -p secret
```

Requires **proxychains-ng** (`proxychains4` on `PATH`, or a compatible `proxychains` that is not legacy v3) and the wrapped command (e.g. `nxc`), unless `PROXYCHAINS_BIN` is set to the ng binary.
