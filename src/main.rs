use std::collections::HashMap;
use std::env;
use std::io::{self, BufRead, Write};
use std::process::exit;

// ps output can carry effectively unbounded command lines (long shell
// invocations, args with embedded data). Capping what we store per
// process keeps one pathological line from blowing up memory the way
// slurping the whole stream into a single String would.
const MAX_COMMAND_LEN: usize = 200;

struct Proc {
    ppid: u32,
    command: String,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let root_filter = parse_args(&args);

    let stdin = io::stdin();
    let mut procs: HashMap<u32, Proc> = HashMap::new();
    let mut order: Vec<u32> = Vec::new();

    // BufRead::lines() pulls one line at a time out of the OS pipe buffer;
    // nothing here ever holds the full input in memory at once, so this
    // works the same whether stdin is a 200-line snapshot or a multi-hour
    // trace being piped in live.
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("pstree-stream: error reading input: {e}");
                exit(1);
            }
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match parse_line(line) {
            Some((pid, ppid, command)) => {
                if !procs.contains_key(&pid) {
                    order.push(pid);
                }
                procs.insert(pid, Proc { ppid, command });
            }
            None => {
                eprintln!("pstree-stream: skipping unparseable line: {line}");
            }
        }
    }

    let children = build_children_index(&procs, &order);

    let roots: Vec<u32> = match root_filter {
        Some(pid) => {
            if procs.contains_key(&pid) {
                vec![pid]
            } else {
                eprintln!("pstree-stream: pid {pid} not found in input");
                exit(1);
            }
        }
        None => order
            .iter()
            .copied()
            .filter(|pid| {
                let ppid = procs[pid].ppid;
                ppid == *pid || !procs.contains_key(&ppid)
            })
            .collect(),
    };

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut visited: HashMap<u32, bool> = HashMap::new();
    for root in roots {
        print_tree(root, &procs, &children, 0, &mut out, &mut visited);
    }
}

fn parse_args(args: &[String]) -> Option<u32> {
    match args.get(1).map(|s| s.as_str()) {
        None => None,
        Some("-h") | Some("--help") => {
            print_usage();
            exit(0);
        }
        Some("--root") => {
            let pid_str = args.get(2).unwrap_or_else(|| {
                eprintln!("pstree-stream: --root requires a pid argument");
                exit(1);
            });
            let pid: u32 = pid_str.parse().unwrap_or_else(|_| {
                eprintln!("pstree-stream: invalid pid '{pid_str}'");
                exit(1);
            });
            Some(pid)
        }
        Some(other) => {
            eprintln!("pstree-stream: unknown argument '{other}'");
            print_usage();
            exit(1);
        }
    }
}

fn print_usage() {
    eprintln!("usage: pstree-stream [--root PID]");
    eprintln!("reads lines of 'PID PPID COMMAND' from stdin and prints a tree");
}

fn parse_line(line: &str) -> Option<(u32, u32, String)> {
    let mut parts = line.split_whitespace();
    let pid: u32 = parts.next()?.parse().ok()?;
    let ppid: u32 = parts.next()?.parse().ok()?;
    let mut command: String = parts.collect::<Vec<_>>().join(" ");
    if command.is_empty() {
        command = "?".to_string();
    }
    if command.len() > MAX_COMMAND_LEN {
        command.truncate(MAX_COMMAND_LEN);
        command.push_str("...");
    }
    Some((pid, ppid, command))
}

fn build_children_index(procs: &HashMap<u32, Proc>, order: &[u32]) -> HashMap<u32, Vec<u32>> {
    let mut children: HashMap<u32, Vec<u32>> = HashMap::new();
    for &pid in order {
        let ppid = procs[&pid].ppid;
        if ppid != pid {
            children.entry(ppid).or_default().push(pid);
        }
    }
    children
}

fn print_tree(
    pid: u32,
    procs: &HashMap<u32, Proc>,
    children: &HashMap<u32, Vec<u32>>,
    depth: usize,
    out: &mut impl Write,
    visited: &mut HashMap<u32, bool>,
) {
    if visited.contains_key(&pid) {
        // malformed input can describe a ppid cycle; stop instead of recursing forever
        let _ = writeln!(out, "{}[{}] <cycle>", "  ".repeat(depth), pid);
        return;
    }
    visited.insert(pid, true);

    let command = procs.get(&pid).map(|p| p.command.as_str()).unwrap_or("?");
    let _ = writeln!(out, "{}[{}] {}", "  ".repeat(depth), pid, command);

    if let Some(kids) = children.get(&pid) {
        for &child in kids {
            print_tree(child, procs, children, depth + 1, out, visited);
        }
    }
}
