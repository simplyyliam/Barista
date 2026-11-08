# 0001: Named pipe for bar and client communication

Status: accepted (2026-11-08, v0.1 Bar shell). The server itself arrives in v0.2.

## Context

Client apps (Electron, Node, later C# and Python) must talk to the one bar process: register
items, send menus, get clicks. The channel has to be local only, work from Node without native
modules, and let the bar know who is calling.

|                                            | Named pipe                     | Localhost TCP / WebSocket    | `WM_COPYDATA`       | COM     |
| ------------------------------------------ | ------------------------------ | ---------------------------- | ------------------- | ------- |
| Reachable from the network                 | no (can reject remote clients) | port is open on loopback     | no                  | no      |
| Other local users or web pages can connect | no (ACL per user)              | yes, any browser tab can try | no                  | depends |
| Works from Node, no native module          | yes (`net`)                    | yes                          | no (needs a window) | no      |
| Bar can see the client's process id        | yes                            | hard                         | yes                 | partly  |
| Setup cost                                 | low                            | low                          | medium              | high    |

## Decision

Use a **named pipe**, `\\.\pipe\barista`, one connection per client, messages are JSON with a
4-byte little-endian length prefix (see `docs/release/plan/03-protocol.md`).

- The pipe is created with `PIPE_REJECT_REMOTE_CLIENTS` and an access list for the current user only.
- The bar reads the client's pid with `GetNamedPipeClientProcessId` and checks it against `hello.pid`.
- Every message is size-checked before it is parsed (CLAUDE.md rule 3).

## Consequences

- No network port, so no firewall prompt and no browser can reach the bar.
- Windows-only, which matches the product.
- A client has to retry until the pipe exists, because the bar may start after it (the SDK does this).
