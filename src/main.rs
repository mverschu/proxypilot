//! proxypilot: select a local SOCKS port from config, write a temporary proxychains
//! configuration, and run `proxychains4 -f <config> <your-command>` with all args forwarded.

use std::env;
use std::fs;
use std::io::{self, stdout, Write};
use std::path::{Path, PathBuf};
use std::process::{self, Command, Stdio};

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::style::Print;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, size, Clear, ClearType, EnterAlternateScreen,
    LeaveAlternateScreen,
};
use crossterm::{execute, queue};
use tempfile::NamedTempFile;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("Usage: proxypilot <command> [args...]");
        eprintln!("Example: proxypilot nxc smb 192.168.1.1 -u user -p pass");
        process::exit(2);
    }

    let config_path = match resolve_config_path() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("proxypilot: {e}");
            process::exit(2);
        }
    };

    let ports = match load_ports(&config_path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("proxypilot: {e}");
            process::exit(2);
        }
    };

    if ports.is_empty() {
        eprintln!("proxypilot: no valid ports in {}", config_path.display());
        process::exit(2);
    }

    let port = match ports.len() {
        1 => ports[0],
        _ => match select_port_tui(&ports) {
            Ok(Some(p)) => p,
            Ok(None) => process::exit(130), // cancelled (Esc)
            Err(e) => {
                eprintln!("proxypilot: {e}");
                process::exit(1);
            }
        },
    };

    let temp_config = match write_proxychains_config(port) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("proxypilot: failed to write temporary config: {e}");
            process::exit(1);
        }
    };

    let path = temp_config.path().to_path_buf();
    let status = Command::new("proxychains4")
        .arg("-f")
        .arg(&path)
        .args(&args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();

    // Deletes the tempfile when dropped.
    drop(temp_config);

    match status {
        Ok(s) => {
            process::exit(s.code().unwrap_or(1));
        }
        Err(e) => {
            eprintln!("proxypilot: failed to run proxychains4: {e}");
            process::exit(1);
        }
    }
}

fn resolve_config_path() -> Result<PathBuf, String> {
    if let Ok(p) = env::var("PROXYPILOT_CONFIG") {
        return Ok(PathBuf::from(p));
    }
    let config_home = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .ok_or_else(|| {
            "set HOME or XDG_CONFIG_HOME, or PROXYPILOT_CONFIG to your ports file".to_string()
        })?;
    Ok(config_home.join("proxypilot").join("ports"))
}

fn load_ports(path: &Path) -> Result<Vec<u16>, String> {
    let content = fs::read_to_string(path).map_err(|e| {
        format!(
            "cannot read config {}: {e}",
            path.display()
        )
    })?;

    let mut ports = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let port: u16 = line
            .parse()
            .map_err(|_| format!("invalid port line in {}: {line:?}", path.display()))?;
        ports.push(port);
    }
    Ok(ports)
}

fn write_proxychains_config(port: u16) -> io::Result<NamedTempFile> {
    let mut f = NamedTempFile::new()?;
    let content = format!(
        "\
strict_chain
quiet_mode
proxy_dns

tcp_read_time_out 15000
tcp_connect_time_out 8000

[ProxyList]
socks5 127.0.0.1 {port}
",
        port = port
    );
    f.write_all(content.as_bytes())?;
    f.flush()?;
    Ok(f)
}

fn select_port_tui(ports: &[u16]) -> io::Result<Option<u16>> {
    let mut selected: usize = 0;

    enable_raw_mode()?;
    let mut out = stdout();
    if let Err(e) = execute!(out, EnterAlternateScreen, Hide) {
        let _ = disable_raw_mode();
        return Err(e);
    }

    let result = (|| -> io::Result<Option<u16>> {
        loop {
            draw_selection(&mut out, ports, selected)?;

            if !event::poll(std::time::Duration::from_millis(250))? {
                continue;
            }
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Esc => return Ok(None),
                    KeyCode::Enter => return Ok(Some(ports[selected])),
                    KeyCode::Up | KeyCode::Char('k') => {
                        if selected > 0 {
                            selected -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if selected + 1 < ports.len() {
                            selected += 1;
                        }
                    }
                    _ => {}
                },
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    })();

    let _ = execute!(out, Show, LeaveAlternateScreen);
    let _ = disable_raw_mode();

    result
}

fn draw_selection(out: &mut impl Write, ports: &[u16], selected: usize) -> io::Result<()> {
    let (_cols, rows) = size().unwrap_or((80, 24));
    queue!(out, Clear(ClearType::All), MoveTo(0, 0))?;

    queue!(out, Print("Select proxy route:\r\n"), Print("\r\n"))?;

    for (i, port) in ports.iter().enumerate() {
        let prefix = if i == selected { "> " } else { "  " };
        queue!(
            out,
            Print(format!("{prefix}127.0.0.1:{port}\r\n"))
        )?;
    }

    queue!(
        out,
        MoveTo(0, rows.saturating_sub(1)),
        Print("↑/↓: move  Enter: confirm  Esc: cancel"),
    )?;

    out.flush()?;
    Ok(())
}
