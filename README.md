# pstree-stream

Turn a flat list of `pid ppid command` records into an indented process
tree.

`ps -eo pid,ppid,comm` gives you a table. Working out who spawned what by
eye is slow once there are more than a couple dozen rows, and on a busy
host or inside a container there can be thousands. pstree-stream reads
its input line by line from stdin - it never reads the whole stream into
one buffer before it starts working - so it behaves the same whether
you're feeding it a one-off snapshot or piping in output continuously
from a long-running trace.

## Usage

```
ps -eo pid=,ppid=,comm= | pstree-stream
```

Example output:

```
[1] init
├─ [412] sshd
│  └─ [8831] bash
│     └─ [9102] pstree-stream
└─ [900] cron
```

Show just one subtree:

```
ps -eo pid=,ppid=,comm= | pstree-stream --root 412
```

Use plain ASCII instead of the Unicode box-drawing characters, for
terminals or fonts that don't render them cleanly:

```
ps -eo pid=,ppid=,comm= | pstree-stream --ascii
```

```
[1] init
|- [412] sshd
|  `- [8831] bash
|     `- [9102] pstree-stream
`- [900] cron
```

## Input format

Each line: `<pid> <ppid> <command>`, whitespace separated. Anything
after the command field is treated as part of the command. Lines that
don't parse are skipped with a warning on stderr, so a stray blank line
or header row from your `ps` invocation won't kill the run.

## Building

```
cargo build --release
```

Standard library only - no third-party dependencies.

## Status

Early skeleton. Output is in input order, not sorted; no --wide flag
yet. See the roadmap in commit history for what's next.
