# todo-flux

A command-line todo list in Rust whose rules are proved at compile time by
[Flux](https://flux-rs.github.io/flux) (liquid types for Rust). It is a small
program on purpose: each part of it shows one thing Flux can do.

```
todo [list]          show all todos
todo list <status>   show only open, doing or done todos
todo add <title>     add a todo
todo start <n>       mark todo n as being worked on
todo done <n>        mark todo n as done
todo undo <n>        mark todo n as open again
todo pri <n> <1-3>   set the priority of todo n (1 is highest)
todo mv <n> <m>      move todo n to position m
todo rm <n>          delete todo n
```

Todos live in `$TODO_FILE` (default `~/.todo.txt`), one per line as
`[ ] !2 title`. The mark is `[ ]` open, `[~]` doing or `[x]` done and `!2` is
the priority, so the file can be edited by hand.

## What Flux proves

Each row is a compile error under Flux when the code gets it wrong, instead of
a runtime panic or a silently wrong file.

| Flux capability | Rule it proves here | Where |
|---|---|---|
| Indexed types, strong references (`ensures`) | An index is below the list's length; `add`, `rm` and `mv` change the length by exactly +1, -1 and 0 | `TodoList` |
| Existential types, type aliases, constants | A priority is always 1 to 3 | `Priority`, `priority` |
| Spec functions (`defs!`) | `locate` and `priority` return `Some` exactly when the input is valid, and exactly the right value | `is_position`, `is_priority` |
| Struct invariants | There are never more done todos than todos, so "left" and "percent" are safe arithmetic | `Progress` |
| Refined arrays | Every priority has a slot in the per-priority counts | `slot`, `open_by_priority` |
| Reflected enums, structs indexed by them | A status changes only as the rules allow: a done todo must be reopened before it is started again | `Status`, `Todo::try_become` |
| Enums whose variants carry data | A parsed command fits the list it was parsed against, so running it needs no checks | `Command`, `parse` |
| Refined closure signatures | The one loop over the list only hands out valid indexes | `TodoList::each_index` |
| `for` loops over ranges, `invariant!` | Loop variables stay in range; one invariant is written by hand | `TodoList::progress` |
| Traits with associated refinements | `list[i]` is in bounds | `impl Index for TodoList` |
| Extern specs | What `Vec::remove` and `Vec::insert` do to the length | `vec_specs` |
| Opaque structs, trusted functions | A title is never blank | `Title` |
| Overflow checking (`strict`, whole crate) | No arithmetic overflows or underflows, no division by zero | `Cargo.toml` |
| `no_panic` | The twelve rule functions cannot panic on any input | `src/list.rs` |
| `should_fail` | Fifteen misuses are rejected, checked on every run | `src/rejected.rs` |

## What Flux does not prove

- Behavior. That `done 2` marks the second todo, that the file round-trips,
  what the messages say. `tests/cli.rs` covers those.
- Assumptions made by hand: the specs for `Vec::remove` and `Vec::insert`,
  and the three trusted functions of `Title`.
- One opt-out: `open_by_priority` is not checked for overflow. Its counts
  cannot pass the list's length, but Flux cannot infer that for every array
  element.
- The functions that touch the `Vec` are not marked `no_panic`: their indexes
  are proved in bounds, but Flux does not accept the standard library's
  indexing and allocation as panic-free.

## Flux features not used

A todo list gave no honest use for these, so they are left out rather than
forced in: sets and bitvectors in refinements, const generics, generic
(abstract) refinements, quantified `forall` preconditions, uninterpreted
functions with hide and reveal, refined function pointers and trait objects,
and detached specs (they move a spec away from the code it describes, which
only pays for code you cannot annotate).

## Build, test, verify

```
cargo build        # stable Rust; the Flux attributes expand to nothing
cargo test         # end-to-end tests of the binary
./verify.sh        # run the Flux verifier
```

`./verify.sh` needs `cargo-flux`, `fixpoint` and `z3`; see the
[Flux install guide](https://flux-rs.github.io/flux/guide/install.html).
