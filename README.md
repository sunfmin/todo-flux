# todo-flux

A command-line todo list in Rust whose rules are proved at compile time by
[Flux](https://flux-rs.github.io/flux) (liquid types for Rust). It is a small
program on purpose: each part of it shows one thing Flux can do.

```
todo [list]          show all todos
todo list <status>.. show only open, doing or done todos
todo next            show the most urgent todo that is not done
todo add <title>     add a todo
todo start <n>       mark todo n as being worked on
todo done <n>        mark todo n as done
todo undo <n>        mark todo n as open again
todo pri <n> <1-3>   set the priority of todo n (1 is highest)
todo mv <n> <m>      move todo n to position m
todo rm <n>..        delete the todos numbered
todo clear           delete every done todo
```

Todos live in `$TODO_FILE` (default `~/.todo.txt`), one per line as
`[ ] !2 title`. The mark is `[ ]` open, `[~]` doing or `[x]` done and `!2` is
the priority, so the file can be edited by hand.

## What Flux proves

Each row is a compile error under Flux when the code gets it wrong, instead of
a runtime panic or a silently wrong file.

| Flux capability | Rule it proves here | Where |
|---|---|---|
| Indexed types, strong references (`ensures`) | An index is below the list's length; `add` and `mv` change the length by exactly +1 and 0 | `TodoList` |
| Existential types, type aliases, constants | A priority is always 1 to 3 | `Priority`, `priority` |
| Spec functions (`defs!`), `let` binders | `locate` and `priority` return `Some` exactly when the input is valid, and exactly the right value | `is_position`, `is_priority`, `need_with` |
| Struct invariants | There are never more done todos than todos, so "left" and "percent" are safe arithmetic | `Progress` |
| Reflected enums, structs indexed by them | A status changes only as the rules allow: a done todo must be reopened before it is started again | `Status`, `Todo::try_become` |
| Record update syntax in specs | Changing a todo's status leaves its priority alone, and the other way round | `Todo::try_become`, `Todo::set_priority` |
| Enums with refined variants, enum invariants | A parsed command fits the list it was parsed against, so running it needs no checks | `Command`, `parse` |
| Refined arrays, const generics, refined function pointers | One counting loop serves any number of slots, and every todo lands in a slot that exists | `TodoList::tally` |
| Refined closure signatures | The loop that hands indexes to a closure only hands out valid ones | `TodoList::each_index` |
| `for` loops over ranges, `invariant!` | Loop variables stay in range; one invariant is written by hand | `TodoList::progress` |
| `qualifier!` | Removal shrinks the list by exactly the todos it returns: none lost, none taken twice | `TodoList::remove_all` |
| Refined slices and iterators | Walking the todos with `enumerate` yields indexes that are in the list | `TodoList::todos`, `next` |
| Associated refinements of a std trait | `list[i]` is in bounds | `impl Index for TodoList` |
| A trait of our own with a defaulted associated refinement | A selection by number must fit the list; one by status fits any list | `Selection` |
| Sets | Which numbers were picked: none can be picked twice | `Picks` |
| Bitvectors, bitvector constants | A set of statuses never holds a bit that is not a status, and behaves like a set | `StatusSet`, `STATUS_BITS` |
| `char` literals, sort casts | The priority in a line is one digit, and it is read as the number it stands for | `priority_digit` |
| String literals in refinements | Each status has its own mark, so a mark in the file says which status it is | `mark_of`, `Status::mark` |
| Uninterpreted functions | A text makes a title exactly when it is not blank, whatever "blank" computes | `blank`, `Title::new` |
| Hidden and revealed definitions | Other proofs rely on the status rule holding, not on what it says | `can_become` |
| `forall` preconditions, bounded quantifiers | Theorems: no status is a dead end; no priority slot is spare | `status_theorems` |
| Generic (abstract) refinements | The smaller of two priorities is a priority, said once for every type | `cmp_specs` |
| Extern specs: methods, a free function, a primitive type | What `Vec::remove`, `Vec::insert`, `std::cmp::min` and `char::to_digit` promise | `vec_specs`, `cmp_specs`, `char_specs` |
| Detached specs, `trusted_derive` | `==` on statuses is true exactly for equal statuses | `Status` |
| Opaque structs, trusted functions | A title is never blank | `Title`, `Picks` |
| Type holes (`_`) | A spec names only the part it is about | `split_priority`, `parse` |
| Refined `impl Trait` and `dyn Trait` | Every priority an iterator yields is a priority; output goes to any writer | `priorities`, `run` |
| `ignore` | The one function with no rule to check is left out | `main` |
| Overflow checking (`strict`, whole crate; `opts` for one function) | No arithmetic overflows or underflows, no division by zero | `Cargo.toml` |
| `no_panic` | The rule functions cannot panic on any input | `src/list.rs` |
| `should_fail` | Each misuse in the file is rejected, checked on every run | `src/rejected.rs` |

## What Flux does not prove

- Behavior. That `done 2` marks the second todo, that the file round-trips,
  what the messages say. `tests/cli.rs` covers those.
- Assumptions made by hand: the extern specs for std, the trusted functions
  of `Title` and `Picks`, and the derived `==` on `Status`. `./verify.sh`
  prints how many functions are trusted.
- One opt-out: `tally` is not checked for overflow. Its counts cannot pass
  the list's length, but Flux cannot infer that for every array element.
- The functions that touch the `Vec` are not marked `no_panic`: their indexes
  are proved in bounds, but Flux does not accept the standard library's
  indexing and allocation as panic-free.

## Flux features not used

These are in the Flux guide and have no use here:

- Properties for primitive operators (`property ... [<<]`). They add facts
  about `<<`, `&` and the like on integers. The one place this program works
  with bits uses bitvectors, which Flux reasons about exactly.
- Uninterpreted operators (`uif_ops`). They make the solver forget what `*`
  or `/` compute, to help it when it gets stuck. It never did, and `percent`
  needs the real arithmetic.
- Scraped qualifiers (`scrape_quals`). An automatic way to find loop
  invariants; the two loops that need one say theirs with `invariant!` and
  `qualifier!`.
- Refined associated types. No trait here has an associated type.
- `RVec` and the other containers in Flux's own test library. `Vec` and
  slices from std, with the specs in `flux-alloc` and `flux-core`, do the job.

To check one function instead of the crate, use Flux's include patterns:
`./verify.sh check --only-check='def:list::TodoList::remove_all'`.

## Build, test, verify

```
cargo build        # stable Rust; the Flux attributes expand to nothing
cargo test         # end-to-end tests of the binary
./verify.sh        # run the Flux verifier
```

`./verify.sh` needs `cargo-flux`, `fixpoint` and `z3`; see the
[Flux install guide](https://flux-rs.github.io/flux/guide/install.html).
