# notifications-spike

Debug-only spike for ADR 0017 (can the bar read Windows notifications?). Not part of the bar or the
Cargo workspace; never shipped.

```text
cd examples/notifications-spike
cargo run -- access | list | watch 60 | dismiss <id> | clear
```
