# todo-flux

A command-line todo list in Rust whose index safety is proved at compile time
by [Flux](https://flux-rs.github.io/flux) (liquid types for Rust).

```
todo [list]          show all todos
todo add <title>     add a todo
todo done <n>        mark todo n as done
todo undo <n>        mark todo n as not done
todo mv <n> <m>      move todo n to position m
todo rm <n>          delete todo n
```

Todos live in `$TODO_FILE` (default `~/.todo.txt`), one per line as
`[ ] title` or `[x] title`, so the file can be edited by hand.

## What Flux proves

Each of these is a compile error under Flux when the code gets it wrong,
instead of a runtime panic or a silently wrong file.

| Property | Where | What a bug would look like |
|---|---|---|
| Indexes are in bounds | `TodoList` carries its length in its type; `get`, `set_done`, `remove` only accept an index below it, and `locate` is the one function that makes such an index from user input | passing an unchecked number, an off-by-one in `locate` |
| No division by zero, no underflow | `percent` requires a non-empty total and a part no larger than it; `done_count` is proved to be at most the length | printing the progress line for an empty list, `done - total` |
| Moving keeps every todo | `move_to` must give back a list of the same length | removing the todo and forgetting to put it back |

Flux does not prove behavior: that `done 2` marks the second todo, that the
file round-trips, or what the error messages say. `tests/cli.rs` covers those.
The specs for `Vec::remove` and `Vec::insert` in `src/list.rs` are assumed,
not proved.

## Build, test, verify

```
cargo build        # stable Rust; the Flux attributes expand to nothing
cargo test         # end-to-end tests of the binary
./verify.sh        # run the Flux verifier
```

`./verify.sh` needs `cargo-flux`, `fixpoint` and `z3`; see the
[Flux install guide](https://flux-rs.github.io/flux/guide/install.html).
