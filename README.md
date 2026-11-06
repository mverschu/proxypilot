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

File format: one port per line (empty lines and `#` comments allowed):

```text
1080
1081
```

- **One port** → used automatically.
- **Several ports** → arrow keys (↑/↓ or `j`/`k`) to move, **Enter** to confirm, **Esc** to cancel.

## Build

```bash
cargo build --release
```

Install the binary from `target/release/proxypilot` into your `PATH`.

## Precompiled binary (Linux x86_64)

A release build for **Linux x86_64** (`x86_64-unknown-linux-gnu`) is checked in under `dist/`:

- **`dist/proxypilot-x86_64-unknown-linux-gnu`** — executable  
- **`dist/proxypilot-x86_64-unknown-linux-gnu.tar.gz`** — same file in a tarball  

This binary is dynamically linked against **glibc** (typical on mainstream distros). Copy it somewhere on your `PATH` (for example `~/.local/bin/`) and ensure it is executable (`chmod +x`). For other targets, build from source with `cargo build --release`.

## Usage

```bash
proxypilot nxc smb 192.168.1.10
proxypilot nxc smb 192.168.1.10 -u alice -p secret
```

Requires `proxychains4` and the wrapped command (e.g. `nxc`) on `PATH`.
