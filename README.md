# proxypilot

<img width="2172" height="724" alt="image" src="https://github.com/user-attachments/assets/0f30eccb-424c-4212-82ae-07554a508f9c" />

Runs any command through **proxychains-ng** using a local SOCKS port from a small config file (optional picker when you have more than one port).

**You need `proxychains4` on your system** (the proxychains-ng package). Other deps: whatever command you wrap (e.g. `nxc`).

## Install

**Option A — prebuilt (Linux x86_64)**  
Grab `proxypilot-linux-x86_64` from [Releases](https://github.com/mverschu/proxypilot/releases), then:

```bash
sudo install -m 755 proxypilot-linux-x86_64 /usr/local/bin/proxypilot
```

*(Use `/usr/bin` instead of `/usr/local/bin` if you prefer.)*

**Option B — build**

```bash
cargo build --release
sudo install -m 755 target/release/proxypilot /usr/local/bin/proxypilot
```

## Usage

**1. Create the config** (writes `~/.config/proxypilot/ports` — ports and optional labels):

```bash
proxypilot init
```

**2. Run a command through the proxy** (args are passed through as-is):

```bash
proxypilot nxc smb 192.168.1.10
proxypilot nxc ldap 10.0.0.5 -u user -p pass
```

<img width="470" height="148" alt="image" src="https://github.com/user-attachments/assets/665e5bfd-8b87-41c3-aa43-658eafeb6e9e" />

One port in the file → it is used automatically. Several ports → pick one in the prompt (↑/↓, Enter, Esc to cancel).
