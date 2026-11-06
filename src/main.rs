//! proxypilot: select a local SOCKS port from config, write a temporary proxychains
//! configuration, and run `proxychains4 -f <config> <your-command>` with all args forwarded.

use std::env;
use std::fs;
use std::io::{self, stdin, stdout, BufRead, Write};
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

/// One SOCKS route from the config file (port plus optional display label).
struct PortRoute {
    port: u16,
    label: Option<String>,
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        print_usage();
        process::exit(2);
    }

    if args[0] == "init" {
        if args.len() != 1 {
            eprintln!("Usage: proxypilot init");
            process::exit(2);
        }
        process::exit(run_init());
    }

    let config_path = match resolve_config_path() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("proxypilot: {e}");
            process::exit(2);
        }
    };

    let routes = match load_routes(&config_path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("proxypilot: {e}");
            process::exit(2);
        }
    };

    if routes.is_empty() {
        eprintln!("proxypilot: no valid ports in {}", config_path.display());
        process::exit(2);
    }

    let port = match routes.len() {
        1 => routes[0].port,
        _ => match select_port_tui(&routes) {
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

fn print_usage() {
    eprintln!("Usage:");
    eprintln!("  proxypilot init              Create ~/.config/proxypilot/ports interactively");
    eprintln!("  proxypilot <command> [args]  Run via proxychains4 through a chosen SOCKS port");
    eprintln!("Example: proxypilot nxc smb 192.168.1.1 -u user -p pass");
}

/// Interactive setup: write the ports file (same path as normal operation).
fn run_init() -> i32 {
    let config_path = match resolve_config_path() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("proxypilot: {e}");
            return 2;
        }
    };

    println!("proxypilot init — create SOCKS port config\n");
    println!("File: {}", config_path.display());

    match fs::metadata(&config_path) {
        Ok(m) if m.is_file() => {
            print!("This file already exists. Overwrite? [y/N]: ");
            let _ = stdout().flush();
            let mut line = String::new();
            if stdin().read_line(&mut line).is_err() {
                return 1;
            }
            let a = line.trim().to_ascii_lowercase();
            if a != "y" && a != "yes" {
                println!("Aborted.");
                return 0;
            }
        }
        Ok(_) => {
            eprintln!("proxypilot: path exists but is not a file: {}", config_path.display());
            return 1;
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => {
            eprintln!("proxypilot: cannot access {}: {e}", config_path.display());
            return 1;
        }
    }

    let stdin = stdin();
    let mut reader = stdin.lock();

    println!("\nAdd one SOCKS port per entry (127.0.0.1 will be used). Empty port when done.\n");

    let mut lines: Vec<String> = Vec::new();
    let mut n: u32 = 1;
    loop {
        print!("Port [{n}] (e.g. 1080, blank to finish): ");
        let _ = stdout().flush();
        let mut port_line = String::new();
        if reader.read_line(&mut port_line).is_err() {
            return 1;
        }
        let port_line = port_line.trim();
        if port_line.is_empty() {
            if lines.is_empty() {
                eprintln!("proxypilot: add at least one port.");
                return 2;
            }
            break;
        }
        let port: u16 = match port_line.parse() {
            Ok(p) => p,
            Err(_) => {
                eprintln!("proxypilot: invalid port: {port_line:?}");
                return 2;
            }
        };

        print!("     Label (optional; leave empty for port only): ");
        let _ = stdout().flush();
        let mut label_line = String::new();
        if reader.read_line(&mut label_line).is_err() {
            return 1;
        }
        let label = label_line.trim();
        lines.push(format_config_line(label, port));
        n += 1;
    }

    if let Some(parent) = config_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            eprintln!("proxypilot: cannot create {}: {e}", parent.display());
            return 1;
        }
    }

    let content = build_ports_file_content(&lines);
    if let Err(e) = fs::write(&config_path, content) {
        eprintln!("proxypilot: cannot write {}: {e}", config_path.display());
        return 1;
    }

    println!("\nWrote {}.", config_path.display());
    0
}

fn format_config_line(label: &str, port: u16) -> String {
    let label = label.trim();
    if label.is_empty() {
        return format!("{port}");
    }
    if label.contains(':') {
        format!("[{label}]:{port}")
    } else {
        format!("{label}:{port}")
    }
}

fn build_ports_file_content(lines: &[String]) -> String {
    let mut s = String::from("# proxypilot — local SOCKS ports (one per line)\n");
    s.push_str("# Plain port, or Label:port / [Label with : colon]:port\n\n");
    for line in lines {
        s.push_str(line);
        s.push('\n');
    }
    s
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

fn load_routes(path: &Path) -> Result<Vec<PortRoute>, String> {
    let content = fs::read_to_string(path).map_err(|e| {
        format!(
            "cannot read config {}: {e}",
            path.display()
        )
    })?;

    let mut routes = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let route = parse_port_line(line).map_err(|msg| {
            format!("invalid line in {}: {msg} ({line:?})", path.display())
        })?;
        routes.push(route);
    }
    Ok(routes)
}

/// Parses `1080`, `Name:1080`, or `[Name]:1080`. Label is optional and shown in the TUI only.
fn parse_port_line(line: &str) -> Result<PortRoute, String> {
    if let Some((label_part, port_str)) = line.rsplit_once(':') {
        if port_str.chars().all(|c| c.is_ascii_digit()) && !port_str.is_empty() {
            let port: u16 = port_str
                .parse()
                .map_err(|_| "invalid port number".to_string())?;
            let label = normalize_label(label_part.trim());
            return Ok(PortRoute {
                port,
                label: if label.is_empty() { None } else { Some(label) },
            });
        }
    }
    let port: u16 = line
        .parse()
        .map_err(|_| "expected a port (e.g. 1080) or label:port (e.g. [Office]:1080)".to_string())?;
    Ok(PortRoute {
        port,
        label: None,
    })
}

fn normalize_label(s: &str) -> String {
    let s = s.trim();
    if s.len() >= 2 && s.starts_with('[') && s.ends_with(']') {
        s[1..s.len() - 1].trim().to_string()
    } else {
        s.to_string()
    }
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

fn select_port_tui(routes: &[PortRoute]) -> io::Result<Option<u16>> {
    let mut selected: usize = 0;

    enable_raw_mode()?;
    let mut out = stdout();
    if let Err(e) = execute!(out, EnterAlternateScreen, Hide) {
        let _ = disable_raw_mode();
        return Err(e);
    }

    let result = (|| -> io::Result<Option<u16>> {
        loop {
            draw_selection(&mut out, routes, selected)?;

            if !event::poll(std::time::Duration::from_millis(250))? {
                continue;
            }
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Esc => return Ok(None),
                    KeyCode::Enter => return Ok(Some(routes[selected].port)),
                    KeyCode::Up | KeyCode::Char('k') => {
                        if selected > 0 {
                            selected -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if selected + 1 < routes.len() {
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

fn draw_selection(out: &mut impl Write, routes: &[PortRoute], selected: usize) -> io::Result<()> {
    let (_cols, rows) = size().unwrap_or((80, 24));
    queue!(out, Clear(ClearType::All), MoveTo(0, 0))?;

    queue!(out, Print("Select proxy route:\r\n"), Print("\r\n"))?;

    for (i, route) in routes.iter().enumerate() {
        let prefix = if i == selected { "> " } else { "  " };
        let line = match &route.label {
            Some(name) => format!("{prefix}{name} - 127.0.0.1:{port}\r\n", port = route.port),
            None => format!("{prefix}127.0.0.1:{port}\r\n", port = route.port),
        };
        queue!(out, Print(line))?;
    }

    queue!(
        out,
        MoveTo(0, rows.saturating_sub(1)),
        Print("↑/↓: move  Enter: confirm  Esc: cancel"),
    )?;

    out.flush()?;
    Ok(())
}
