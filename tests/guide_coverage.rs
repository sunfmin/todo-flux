//! Keeps the README's claim honest: every section of the Flux specifications
//! guide (https://flux-rs.github.io/flux/guide/specifications.html) names a
//! line of this program that uses it. If a refactor removes the last use of
//! a capability, this test fails and says which.
//!
//! Finding the text only shows the capability is written down. That it is
//! exercised is `./verify.sh`'s job: Flux checks every one of these lines.

use std::fs;

/// Guide section, file, and text of a line that uses the capability.
const USES: &[(&str, &str, &str)] = &[
    (
        "Refinement Types",
        "src/main.rs",
        "Result<usize{i: i < n}, _>",
    ),
    (
        "Argument Syntax",
        "src/list.rs",
        "fn(n: u8) -> Option<u8[n]>",
    ),
    (
        "Grammar of Refinements",
        "src/list.rs",
        "HIGHEST_PRIORITY <= p && p <= LOWEST_PRIORITY",
    ),
    (
        "Index Refinements",
        "src/list.rs",
        "fn(&TodoList[@n]) -> usize[n]",
    ),
    ("Existential Refinements", "src/list.rs", "usize{i: i < n}"),
    (
        "Combining Index and Existential Refinements",
        "src/list.rs",
        "{u8[priority] | is_priority(priority)}",
    ),
    (
        "Mutable References",
        "src/list.rs",
        "fn(&mut TodoList[@n], usize{i: i < n}, to: Status) -> bool",
    ),
    (
        "Strong References",
        "src/list.rs",
        "ensures self: TodoList[n + 1]",
    ),
    (
        "Mixing Mutable and Strong References",
        "src/list.rs",
        "self.items.get_mut(index).try_become(to)",
    ),
    ("Refined Arrays", "src/list.rs", "-> [usize; N])]"),
    (
        "Refined Vectors `rvec`",
        "src/rvec.rs",
        "pub struct RVec<T>",
    ),
    ("Refined Slices", "src/list.rs", "-> &[Todo][n]"),
    (
        "Refined `Vec`",
        "src/main.rs",
        "Vec<usize{i: first <= i && i <= last}>[last - first + 1]",
    ),
    ("Named Function Signatures", "src/list.rs", "#[spec(fn("),
    ("Requires Clauses", "src/list.rs", "requires p.total > 0"),
    (
        "Refining Structs",
        "src/list.rs",
        "#[refined_by(done: int, total: int)]",
    ),
    (
        "Invariants on Structs",
        "src/list.rs",
        "#[invariant(done <= total)]",
    ),
    ("Opaque Structs", "src/list.rs", "#[opaque]"),
    (
        "Refining Enums",
        "src/main.rs",
        "#[variant((usize[@index], Status) -> Command[index + 1])]",
    ),
    (
        "Invariants on Enums",
        "src/main.rs",
        "#[invariant(need >= 0)]\nenum Command",
    ),
    ("Reflecting Enums", "src/list.rs", "#[reflect]"),
    (
        "Field Syntax for Indices: Structs",
        "src/list.rs",
        "can_become(t.status, to)",
    ),
    (
        "Field Syntax for Indices: Enums",
        "src/main.rs",
        "Command{c: c.need <= n}",
    ),
    (
        "Field Syntax for Indices: Updates",
        "src/list.rs",
        "Todo { priority: priority, ..t }",
    ),
    (
        "Const",
        "src/list.rs",
        "usize{slot: slot < PRIORITY_LEVELS}",
    ),
    (
        "Requires with `forall`",
        "src/list.rs",
        "requires forall s. can_become(s, Status::Open)",
    ),
    (
        "Refined Associated Types",
        "src/list.rs",
        "fn(&Self[@selection], Self::By[@by])",
    ),
    (
        "Ignored and trusted code",
        "src/main.rs",
        "#[flux_rs::ignore]",
    ),
    ("Ignored and trusted code", "src/list.rs", "#[trusted]"),
    ("Derive-generated code", "src/list.rs", "#[trusted_derive("),
    ("Pragma: `should_fail`", "src/rejected.rs", "#[should_fail]"),
    ("Const Generics", "src/list.rs", "fn tally<const N: usize>"),
    (
        "Type Aliases",
        "src/list.rs",
        "#[alias(type Priority = u8{p: is_priority(p)})]",
    ),
    (
        "Spec Function Definitions: Plain Expressions",
        "src/list.rs",
        "fn is_position(pos: int, len: int) -> bool",
    ),
    (
        "Spec Function Definitions: `let` binders",
        "src/list.rs",
        "let past = index + 1;",
    ),
    (
        "Spec Function Definitions: Bounded Quantification",
        "src/list.rs",
        "forall slot in 0 .. 3",
    ),
    (
        "Spec Function Definitions: Importing Flux Definitions",
        "src/main.rs",
        "use crate::list::need_with;",
    ),
    (
        "Uninterpreted Function Declarations",
        "src/list.rs",
        "fn blank(text: str) -> bool;",
    ),
    (
        "Hiding and Revealing Function Definitions",
        "src/list.rs",
        "#[hide]",
    ),
    (
        "Hiding and Revealing Function Definitions",
        "src/list.rs",
        "#[reveal(can_become)]",
    ),
    (
        "Type Holes: Function Signatures",
        "src/list.rs",
        "fn(_) -> (Priority, _)",
    ),
    (
        "Type Holes: Structs and Enums",
        "src/list.rs",
        "#[field(BTreeSet<_, _>[picked])]",
    ),
    (
        "Type Holes: Structs and Enums",
        "src/main.rs",
        "#[variant((_) -> Command[0])]",
    ),
    (
        "Type Holes: Type Aliases",
        "src/list.rs",
        "type Removed[taken: int] = Vec<_>[taken]",
    ),
    (
        "Type Holes: Generic Args",
        "src/main.rs",
        "Result<Priority, _>",
    ),
    ("Closures", "src/list.rs", "where F: FnMut(usize{i: i < n})"),
    (
        "Function Pointers",
        "src/list.rs",
        "slot_of: fn(&Todo) -> Option<usize{slot: slot < N}>",
    ),
    (
        "Traits and Implementations",
        "src/list.rs",
        "impl Selection for Picks",
    ),
    (
        "Impl Trait",
        "src/list.rs",
        "fn() -> impl Iterator<Item = Priority>",
    ),
    (
        "Dynamic Trait Objects",
        "src/main.rs",
        "Show(&'static dyn Choice)",
    ),
    (
        "Generic Refinements",
        "src/list.rs",
        "fn[hrn p: T -> bool](T{v: p(v)}, T{v: p(v)}) -> T{v: p(v)}",
    ),
    (
        "Bitvector Refinements: Operators",
        "src/list.rs",
        "bits & STATUS_BITS == bits",
    ),
    (
        "Bitvector Refinements: Specification functions",
        "src/list.rs",
        "bv_int_to_bv32(7)",
    ),
    (
        "Bitvector Refinements: Extensions",
        "src/list.rs",
        "bv_zero_extend_32_to_64(bits)",
    ),
    (
        "Bitvector Refinements: Bitvector Constants",
        "src/list.rs",
        "#[constant(bv_int_to_bv32(7))]",
    ),
    ("`char` Literals", "src/list.rs", "'0' <= c && c <= '9'"),
    (
        "`String` Literals",
        "src/list.rs",
        "if status == Status::Open { \"[ ] \" }",
    ),
    (
        "Extern Specs: Extern Functions",
        "src/list.rs",
        "fn min<T: Ord>(a: T, b: T) -> T;",
    ),
    (
        "Extern Specs: Options",
        "src/list.rs",
        "Option<u8[n]>[is_priority(n)]",
    ),
    ("Extern Specs: Vec", "src/list.rs", "removed.push("),
    (
        "Extern Specs: Extern Structs",
        "src/list.rs",
        "struct BTreeSet<T, A: AllocatorClone>;",
    ),
    (
        "Extern Specs: Extern Impls",
        "src/list.rs",
        "impl<T, A: AllocatorClone> BTreeSet<T, A>",
    ),
    (
        "Extern Specs: Extern Traits",
        "src/list.rs",
        "trait Default {",
    ),
    (
        "`for` loops with range `i..j`",
        "src/list.rs",
        "for i in 0..self.len()",
    ),
    (
        "Associated Refinements: Basic",
        "src/list.rs",
        "#[assoc(fn holds(selection: Self, by: Self::By) -> bool)]",
    ),
    (
        "Associated Refinements: Check Subtyping at Impl",
        "src/list.rs",
        "#[assoc(fn holds(picks: Picks, index: int) -> bool",
    ),
    (
        "Associated Refinements: Default",
        "src/list.rs",
        "#[assoc(fn need(selection: Self) -> int { 0 })]",
    ),
    (
        "Associated Refinements: Use in Extern Spec",
        "src/list.rs",
        "<TodoList as Index<usize>>::in_bounds(list, index)",
    ),
    (
        "Associated Refinements: Associated Types",
        "src/list.rs",
        "type By = Status;",
    ),
    (
        "Checking Overflows",
        "Cargo.toml",
        "check_overflow = \"strict\"",
    ),
    (
        "Extensible Properties for Primitive Ops",
        "src/list.rs",
        "property OneShiftedToASlot[<<](x, y)",
    ),
    ("Casting Sorts", "src/list.rs", "cast(c) - cast('0')"),
    ("Detached Specifications", "src/list.rs", "#[specs {"),
    (
        "Include Patterns",
        "Cargo.toml",
        "include_trusted = [\"src/rvec.rs\"]",
    ),
    ("Include Patterns", "verify.sh", "--only-check=\"def:$1\""),
    (
        "Scraping Qualifiers",
        "src/main.rs",
        "#[opts(scrape_quals = \"true\")]",
    ),
    (
        "Invariant Macro",
        "src/main.rs",
        "invariant!(width: int, rest: int;",
    ),
    (
        "Qualifier Macro",
        "src/list.rs",
        "qualifier!(left: int, taken: int, all: int;",
    ),
    ("Uninterpreted Operators", "src/list.rs", "uif_ops = \"+\""),
];

#[test]
fn every_guide_section_names_a_line_that_uses_it() {
    let missing: Vec<String> = USES
        .iter()
        .filter(|(_, file, text)| !fs::read_to_string(file).unwrap().contains(text))
        .map(|(section, file, text)| format!("{section}: `{text}` is no longer in {file}"))
        .collect();
    assert!(missing.is_empty(), "{}", missing.join("\n"));
}
