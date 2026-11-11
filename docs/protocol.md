# Protocols

## The control socket

Every running machine listens on a Unix socket:

```text
/tmp/vista<uid>/<first 8 characters of the machine id>.sock
```

The directory is created mode 0700. Sockets live there rather than in the
machine's folder because a socket path is capped at 104 bytes and bundle
paths can be longer than that.

A client sends one JSON object per line and reads one JSON object per line
back. A connection can carry any number of requests. A machine with no socket,
or nobody answering on it, is not running. Requests are tagged by `op`,
answers by `result`:

```sh
printf '%s\n' '{"op":"status"}' | nc -U /tmp/vista501/3d2063d9.sock
{"result":"status","state":"running","progress":null,"message":null,"pid":96848,"uptime":42,"ip":"192.168.64.7","visible":true}
```

### Requests

| `op` | Fields | Does |
| --- | --- | --- |
| `status` | | Answers `status`. |
| `show` | | Brings the window forward, creating it for a machine started hidden. |
| `hide` | | Hides the window; the machine keeps running. |
| `stop` | | Presses the guest's power button. |
| `kill` | | Stops the machine at once; the process exits. |
| `pause` | | Freezes the guest. |
| `resume` | | Continues a paused guest. |
| `suspend` | | Saves memory to the bundle, then the process exits. |
| `restart` | | Asks the guest to shut down, then starts it again with the configuration on disk. Falls back to a hard stop. |
| `snapshot` | `name` | Answers `snapshot`. Running machines include memory. |
| `screenshot` | | Answers `image`. Needs the window on screen. |
| `key` | `keys` | Presses keys: `"cmd+c"`, `"ctrl+alt+delete"`, several separated by spaces. |
| `type` | `text` | Types text on a US layout. Errors list characters that have no key. |
| `mouse` | `action`, `x`, `y`, `to` | `action` is `click`, `double`, `right`, `move`, `down`, `up` or `drag`; `to` is `[x, y]` for a drag. Screenshot pixels, origin top-left. |
| `scroll` | `x`, `y`, `dx`, `dy` | Pixels; positive `dy` scrolls down. |
| `guest` | `request` | Relays a Vista Tools request (below). Answers `guest`. |

### Answers

| `result` | Fields |
| --- | --- |
| `ok` | |
| `status` | `state`, `progress` (0–1 while installing macOS), `message`, `pid`, `uptime` (seconds), `ip`, `visible` |
| `snapshot` | `id`, `name`, `created` (epoch seconds), `live` (has memory) |
| `image` | `png` (base64), `width`, `height` |
| `guest` | `answer`: the Vista Tools answer |
| `error` | `message`, meant for a person |

`state` is one of `stopped`, `suspended`, `starting`, `running`, `pausing`,
`paused`, `resuming`, `stopping`, `saving`, `restoring`, `installing`,
`error`.

Clients time out after 15 seconds, longer for requests that take longer:
5 minutes for `suspend` and `snapshot`, a command's own timeout plus 15
seconds for `exec`, 2 minutes for other guest requests.

## Vista Tools

`vistatools` listens on virtio-vsock port **7000** inside the guest. Each
connection carries one request line and one answer line. File contents are
base64, at most 64 MiB per request.

### Requests

| `op` | Fields |
| --- | --- |
| `hello` | |
| `exec` | `command`, `shell` (default true: `/bin/sh -c`), `cwd`, `env` (object), `stdin`, `timeout` (seconds, default 60), `admin` (run as root) |
| `read` | `path`, `offset`, `length` |
| `write` | `path`, `data` (base64), `append`, `mode` (octal string, e.g. `"755"`) |
| `list` | `path` |
| `stat` | `path` |
| `mkdir` | `path` (parents created) |
| `remove` | `path`, `recursive` |

### Answers

| `result` | Fields |
| --- | --- |
| `hello` | `version`, `os`, `arch`, `hostname`, `user`, `kernel` |
| `exec` | `code` (null when killed by a signal), `stdout`, `stderr` (1 MB each), `timedout`, `elapsed` (ms) |
| `data` | `data` (base64), `size` (whole file), `eof` |
| `entries` | `entries`: a list of file info |
| `info` | file info: `name`, `path`, `kind` (`file`, `dir`, `link`, `other`), `size`, `modified` (epoch seconds), `mode` (octal) |
| `ok` | |
| `error` | `message` |

Relayed through the control socket, a Vista Tools answer arrives wrapped:
`{"result":"guest","answer":{"result":"exec",…}}`. Both protocols tag on
`result`, so the inner answer has a field of its own.

## The MCP server

`vistamcp` speaks MCP over stdio — JSON-RPC 2.0, one message per line — and
supports protocol versions `2025-06-18`, `2025-03-26` and `2024-11-05`. It
implements `initialize` (with instructions for the agent), `ping`,
`tools/list` and `tools/call`, and answers `resources/list` and
`prompts/list` with empty lists. Tool results carry text, structured content,
and PNG images for screenshots. See the [tool reference](tools.md).
