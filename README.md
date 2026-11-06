# proxypilot

Wrapper around **proxychains4**: reads local SOCKS proxy ports from a config file, optionally lets you pick one in a small TUI, writes a **temporary** proxychains configuration, and runs:

```text
proxychains4 -f <temporary-config> <your-command> [args...]
```

All arguments after `proxypilot` are forwarded unchanged. The child process’s exit code is returned. The temporary file is removed after the command finishes; nothing under `/etc/proxychains.conf` is modified.

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
- **Several routes** → arrow keys (↑/↓ or `j`/`k`) to move, **Enter** to confirm, **Esc** to cancel.

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

Requires `proxychains4` and the wrapped command (e.g. `nxc`) on `PATH`.
