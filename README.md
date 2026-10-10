# todo-flux

A command-line todo list in Rust whose rules are proved at compile time by
[Flux](https://flux-rs.github.io/flux) (liquid types for Rust). It is a small
program on purpose: each part of it shows one thing Flux can do.

```
todo [list]          show all todos
todo list <status>.. show only open, doing or done todos
todo next            show the most urgent todo that is not done
todo now             show the todo being worked on
todo add <title>     add a todo
todo start <n>       mark todo n as being worked on
todo done <n>        mark todo n as done
todo undo <n>        mark todo n as open again
todo pri <n> <1-3>   set the priority of todo n (1 is highest)
todo mv <n> <m>      move todo n to position m
todo rm <n|n-m>..    delete the todos numbered; 2-4 means 2, 3 and 4
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
| `for` loops over ranges | Loop variables stay in range, with nothing written by hand | `TodoList::progress` |
| `invariant!` | Counting the digits of the largest todo number cannot overflow | `number_width` |
| `qualifier!` | Removal shrinks the list by exactly the todos it returns: none lost, none taken twice | `TodoList::remove_all` |
| Scraped qualifiers (`scrape_quals`) | A range like `2-4` names every number between its ends and exactly that many | `span` |
| Refined slices and iterators | Walking the todos with `enumerate` yields indexes that are in the list | `TodoList::todos`, `next` |
| Associated refinements of a std trait | `list[i]` is in bounds | `impl Index for TodoList` |
| A trait of our own with a defaulted associated refinement | A selection by number must fit the list; one by status fits any list | `Selection` |
| Refined associated types | What a selection holds is said over the thing it chooses by, a number or a status, and each implementation is checked against it | `Selection::By` |
| Sets | Which numbers were picked: none can be picked twice | `Picks` |
| Bitvectors, bitvector constants and extensions | A set of statuses never holds a bit that is not a status, behaves like a set, and stays one in a wider word | `StatusSet`, `STATUS_BITS`, `status_set_theorems` |
| Properties for primitive operators | A status's bit is `1 << slot`, so the bits follow the slots | `OneShiftedToASlot`, `Status::bit` |
| Uninterpreted operators (`uif_ops`) | The counting loop's indexes are proved in bounds with `+` treated as unknown, so the proof does not lean on its unchecked additions; the percent bound, by contrast, is unprovable without `*` and `/` | `TodoList::tally`, `src/rejected.rs` |
| `char` literals, sort casts | The priority in a line is one digit, and it is read as the number it stands for | `priority_digit` |
| String literals in refinements | Each status has its own mark, so a mark in the file says which status it is | `mark_of`, `Status::mark` |
| Uninterpreted functions | A text makes a title exactly when it is not blank, whatever "blank" computes | `blank`, `Title::new` |
| Hidden and revealed definitions | Other proofs rely on the status rule holding, not on what it says | `can_become` |
| `forall` preconditions, bounded quantifiers | Theorems: no status is a dead end; no priority slot is spare | `status_theorems` |
| Generic (abstract) refinements | The more urgent of two priorities is a priority, from a spec that never mentions priorities: whatever both numbers satisfy, the answer does | `more_urgent`, `TodoList::most_urgent_left` |
| Extern specs: a struct and its methods, a free function, a primitive type, a trait | What `BTreeSet`, `std::cmp::max`, `char::to_digit` and `Default` promise; a todo starts open | `set_specs`, `cmp_specs`, `char_specs`, `default_specs` |
| Include patterns | The refined vector's file is the trusted base, named once | `Cargo.toml` (`include_trusted`) |
| A refined vector (`RVec`) | Reading a todo at a proved index cannot panic | `src/rvec.rs` |
| Detached specs, `trusted_derive` | Specs stated beside the code and not on it: for a derived `==` (trait impl), a function, an enum, a struct and its impl, a trait | `Status`, `indexes_of`, `Numbers`, `StatusSet`, `Choice` |
| Opaque structs, trusted functions | A title is never blank | `Title`, `RVec` |
| Type holes (`_`) in signatures, fields, variants and aliases | A spec names only the part it is about | `split_priority`, `parse`, `Picks`, `Command`, `Removed` |
| Refined `impl Trait` | Every priority an iterator yields is a priority | `priorities` |
| Refined `dyn Trait` | `next` and `now` choose a todo through a trait object, and the index that comes back is proved to be in the list | `Choice` |
| `ignore` | The one function with no rule to check is left out | `main` |
| Overflow checking (`strict`, whole crate; `opts` for one function) | No arithmetic overflows or underflows, no division by zero | `Cargo.toml` |
| `no_panic` | The rule functions cannot panic on any input | `src/list.rs` |
| `should_fail` | Each misuse in the file is rejected, checked on every run | `src/rejected.rs` |

## What Flux does not prove

- Behavior. That `done 2` marks the second todo, that the file round-trips,
  what the messages say. `tests/cli.rs` covers those.
- Assumptions made by hand: the extern specs for std, the trusted functions
  of `Title` and `RVec`, `Picks::contains`, and the derived `==` on `Status`.
  `./verify.sh` prints how many functions are trusted.
- One opt-out: `tally` is not checked for overflow. Its counts cannot pass
  the list's length, but Flux cannot infer that for every array element.
- The functions that add or remove todos are not marked `no_panic`: growing
  a vector can fail to allocate.

## Coverage of the Flux guide

The table follows the sections of the
[specifications guide](https://flux-rs.github.io/flux/guide/specifications.html);
each has a use in the code, and a proof that rests on that use.
`mutants.py` is the check. It first reads the guide itself, at the Flux
revision `Cargo.toml` pins, and requires every heading and named sub-item
in it to have a mutant, or to be listed in `NOT_REFLECTED` with the reason:
98 sections, 8 of them not reflected (three example programs, a remark, a
restriction, a part the guide leaves as TODO, and two forms the code has no
place for: a detached spec of an enum and a const parameter in a struct
field). Then for every section it breaks the one line that uses the
capability, runs Flux, and expects the program to be rejected. A
line that can be broken without Flux noticing is doing no work, and a line
that is gone is reported too, so a refactor cannot quietly drop a
capability.

```
uv run mutants.py             # the sections, then all 84 mutants: about six minutes
uv run mutants.py Sets        # only the sections whose name contains "Sets"
uv run mutants.py --sections  # only the comparison with the guide, one second
```

Three things it is fair to say about what that shows:

- Flux proves at compile time. The theorems (`status_theorems`,
  `status_set_theorems`, `picks_theorems`, `title_theorems`) and the
  rejected examples in `src/rejected.rs` are compiled only under Flux; the
  program does not call them, and does not need to.
- `uif_ops` can only take knowledge away from the solver, so it proves
  nothing new. It shows which proofs depend on arithmetic.
- Removing `#[flux_rs::ignore]` from `main` breaks nothing, since Flux can
  check `main`. That mutant is caught by Flux's count of ignored functions
  instead.

Include patterns also work from the command line: `./verify.sh remove_all`
checks only the functions whose path contains that name.

## Build, test, verify

```
cargo build        # stable Rust; the Flux attributes expand to nothing
cargo test         # end-to-end tests of the binary
./verify.sh        # run the Flux verifier
./verify.sh span   # verify one function by name
```

`./verify.sh` needs `cargo-flux`, `fixpoint` and `z3`; see the
[Flux install guide](https://flux-rs.github.io/flux/guide/install.html).
