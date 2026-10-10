# todo-flux

A command-line todo list in Rust whose index safety is proved at compile time
by [Flux](https://flux-rs.github.io/flux) (liquid types for Rust).

```
todo [list]        show all todos
todo add <title>   add a todo
todo done <n>      mark todo n as done
todo undo <n>      mark todo n as not done
todo rm <n>        delete todo n
```

Todos live in `$TODO_FILE` (default `~/.todo.txt`), one per line as
`[ ] title` or `[x] title`, so the file can be edited by hand.

## What Flux proves

`TodoList` carries its length in its type (`src/list.rs`). `get`, `set_done`
and `remove` only accept an index Flux can prove is below that length, and
`locate` is the one function that turns a number typed by the user into such
an index. Passing an unchecked number, or an off-by-one in `locate`, is a
compile error under Flux rather than a runtime panic.

## Build, test, verify

```
cargo build        # stable Rust; the Flux attributes expand to nothing
cargo test         # end-to-end tests of the binary
./verify.sh        # run the Flux verifier
```

`./verify.sh` needs `cargo-flux`, `fixpoint` and `z3`; see the
[Flux install guide](https://flux-rs.github.io/flux/guide/install.html).
