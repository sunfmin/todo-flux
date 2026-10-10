"""Shows each Flux capability is exercised, not just written down.

For every section of the Flux specifications guide this breaks the line that
uses the capability and runs the verifier: Flux must then reject the program.
A capability whose line can be broken without Flux noticing is not doing any
work, and is reported as SURVIVED. So is one whose line is gone, which keeps
a refactor from quietly dropping the last use of a capability.

    uv run mutants.py            run every mutant (a few minutes)
    uv run mutants.py Sets       run the mutants whose section contains "Sets"
"""

import pathlib
import re
import subprocess
import sys

# For a mutant with nothing in it to reject: Flux must instead stop counting a
# function it used to count as ignored.
UNCOUNTED = "uncounted"

# Guide section, file, the text to break, what to break it into.
MUTANTS = [
    ("Refinement Types", "src/main.rs", "Result<usize{i: i < n}, _>", "Result<usize{i: i <= n}, _>"),
    ("Argument Syntax", "src/list.rs", "fn(n: u8) -> Option<u8[n]>[is_priority(n)]", "fn(n: u8) -> Option<u8[n]>[true]"),
    ("Grammar of Refinements", "src/list.rs", "HIGHEST_PRIORITY <= p && p <= LOWEST_PRIORITY }", "HIGHEST_PRIORITY <= p || p <= LOWEST_PRIORITY }"),
    ("Index Refinements", "src/list.rs", "fn(&TodoList[@n]) -> usize[n])]", "fn(&TodoList[@n]) -> usize[n + 1])]"),
    ("Existential Refinements", "src/list.rs", "fn(p: Priority) -> usize{slot: slot < PRIORITY_LEVELS}", "fn(p: Priority) -> usize{slot: slot + 1 < PRIORITY_LEVELS}"),
    ("Combining Index and Existential Refinements", "src/list.rs", "{u8[priority] | is_priority(priority)}", "{u8[priority] | true}"),
    ("Mutable References", "src/list.rs", "fn(&mut TodoList[@n], usize{i: i < n}, to: Status) -> bool", "fn(&mut TodoList[@n], usize{i: i <= n}, to: Status) -> bool"),
    ("Strong References", "src/list.rs", "fn(self: &mut TodoList[@n], Todo) ensures self: TodoList[n + 1]", "fn(self: &mut TodoList[@n], Todo) ensures self: TodoList[n + 2]"),
    ("Mixing Mutable and Strong References", "src/list.rs", "self.items.get_mut(index).try_become(to)", "self.items.get_mut(index + 1).try_become(to)"),
    ("Refined Arrays", "src/list.rs", "counts[slot] += 1;", "counts[slot + 1] += 1;"),
    ("Refined Vectors `rvec`", "src/rvec.rs", "fn(&RVec<T>[@n], index: usize{index < n}) -> &T", "fn(&RVec<T>[@n], index: usize{index + 1 < n}) -> &T"),
    ("Refined Slices", "src/list.rs", "-> &[Todo][n])]", "-> &[Todo][n + 1])]"),
    ("Refined `Vec`", "src/main.rs", "[last - first + 1])]", "[last - first + 2])]"),
    ("Named Function Signatures", "src/list.rs", "fn(&TodoList[@n], index: usize{index < n}) -> usize[index + 1]", "fn(&TodoList[@n], index: usize{index < n}) -> usize[index]"),
    ("Requires Clauses", "src/list.rs", "requires p.total > 0", "requires p.total >= 0"),
    ("Refining Structs", "src/list.rs", "#[field({usize[done] | done <= total})]", "#[field(usize)]"),
    ("Invariants on Structs", "src/list.rs", "#[invariant(done <= total)]", "#[invariant(done >= total)]"),
    ("Opaque Structs", "src/list.rs", "#[opaque]\n", ""),
    ("Refining Enums", "src/main.rs", "(usize[@index], Status) -> Command[index + 1]", "(usize[@index], Status) -> Command[index]"),
    ("Invariants on Enums", "src/main.rs", "#[invariant(need >= 0)]\nenum Command", "#[invariant(need > 0)]\nenum Command"),
    ("Reflecting Enums", "src/list.rs", "#[reflect]\n", ""),
    ("Field Syntax for Indices: Structs", "src/list.rs", "status: if can_become(t.status, to) { to } else { t.status }", "status: if can_become(t.status, to) { t.status } else { to }"),
    ("Field Syntax for Indices: Enums", "src/main.rs", "Command{c: c.need <= n}", "Command{c: c.need <= n + 1}"),
    ("Field Syntax for Indices: Updates", "src/list.rs", "Todo { priority: priority, ..t }", "Todo { priority: 1, ..t }"),
    ("Const", "src/list.rs", "pub const LOWEST_PRIORITY: u8 = 3;", "pub const LOWEST_PRIORITY: u8 = 4;"),
    ("Requires with `forall`", "src/list.rs", "requires forall s. can_become(s, Status::Open) && can_become(s, Status::Done)", "requires forall s. can_become(s, Status::Open) && can_become(s, Status::Doing)"),
    ("Refined Associated Types", "src/list.rs", "-> bool[<Self as Selection>::holds(selection, by)]", "-> bool[!<Self as Selection>::holds(selection, by)]"),
    ("Ignored and trusted code: trusted", "src/list.rs", "    #[trusted]\n    #[spec(fn(&str[@text]) -> Option<Title>[!blank(text)])]", "    #[spec(fn(&str[@text]) -> Option<Title>[!blank(text)])]"),
    ("Ignored and trusted code: ignore", "src/main.rs", "#[flux_rs::ignore]\n", "", UNCOUNTED),
    ("Derive-generated code", "src/list.rs", '#[trusted_derive(reason = "Flux cannot check the discriminant comparison derive writes")]\n', ""),
    ("Pragma: `should_fail`", "src/rejected.rs", "#[should_fail]\n", ""),
    ("Const Generics", "src/list.rs", "let mut counts = [0; N];", "let mut counts = [0; 2];"),
    ("Type Aliases", "src/list.rs", "#[alias(type Priority = u8{p: is_priority(p)})]", "#[alias(type Priority = u8{p: p <= LOWEST_PRIORITY})]"),
    ("Spec Function Definitions: Plain Expressions", "src/list.rs", "{ 1 <= pos && pos <= len }", "{ 0 <= pos && pos <= len }"),
    ("Spec Function Definitions: `let` binders", "src/list.rs", "let past = index + 1;\n        if past > need", "let past = index;\n        if past > need"),
    ("Spec Function Definitions: Bounded Quantification", "src/list.rs", "forall slot in 0 .. 3", "forall slot in 0 .. 4"),
    ("Spec Function Definitions: Importing Flux Definitions", "src/main.rs", "    use crate::list::need_with;\n", ""),
    ("Uninterpreted Function Declarations", "src/list.rs", "fn blank(text: str) -> bool;", "fn blank(text: str) -> bool { false }"),
    ("Hiding and Revealing Function Definitions: hide", "src/list.rs", "    #[hide]\n", ""),
    ("Hiding and Revealing Function Definitions: reveal", "src/list.rs", "    #[reveal(can_become)]\n", ""),
    ("Type Holes: Function Signatures", "src/list.rs", "fn(_) -> (Priority, _)", "fn(_) -> (u8, _)"),
    ("Type Holes: Structs and Enums: struct", "src/list.rs", "#[field(BTreeSet<_, _>[picked])]", "#[field(BTreeSet<_, _>[set_empty(0)])]"),
    ("Type Holes: Structs and Enums: enum", "src/main.rs", "#[variant((_) -> Command[0])]", "#[variant((_) -> Command[1])]"),
    ("Type Holes: Type Aliases", "src/list.rs", "= Vec<_>[taken])]", "= Vec<_>[taken + 1])]"),
    ("Type Holes: Generic Args", "src/main.rs", "#[spec(fn(_) -> Result<Priority, _>)]", "#[spec(fn(_) -> Result<u8, _>)]"),
    ("Closures", "src/list.rs", "where F: FnMut(usize{i: i < n})", "where F: FnMut(usize{i: i <= n})"),
    ("Function Pointers", "src/list.rs", "slot_of: fn(&Todo) -> Option<usize{slot: slot < N}>", "slot_of: fn(&Todo) -> Option<usize{slot: slot <= N}>"),
    ("Traits and Implementations", "src/list.rs", "    fn has(&self, index: usize) -> bool {\n        self.contains(index)", "    fn has(&self, index: usize) -> bool {\n        !self.contains(index)"),
    ("Impl Trait", "src/list.rs", "#[spec(fn() -> impl Iterator<Item = Priority>)]\npub fn priorities() -> impl Iterator<Item = Priority>", "pub fn priorities() -> impl Iterator<Item = u8>"),
    ("Dynamic Trait Objects", "src/list.rs", "#[spec(fn(&Self, &TodoList[@n]) -> Option<usize{i: i < n}>)]\n    fn of(&self, list: &TodoList) -> Option<usize>;", "#[spec(fn(&Self, &TodoList[@n]) -> Option<usize{i: i <= n}>)]\n    fn of(&self, list: &TodoList) -> Option<usize>;"),
    ("Generic Refinements", "src/list.rs", "-> u8{v: p(v) && v <= a && v <= b}", "-> u8{v: v <= a && v <= b}"),
    ("Bitvector Refinements: Operators", "src/list.rs", "bits & STATUS_BITS == bits", "bits & STATUS_BITS == 0"),
    ("Bitvector Refinements: Specification functions", "src/list.rs", "bool[bits & bit_of(status) != 0])]\n    pub fn contains", "bool[bits & bit_of(status) == 0])]\n    pub fn contains"),
    ("Bitvector Refinements: Extensions", "src/list.rs", "bv_zero_extend_32_to_64(bits) & 7 ==", "bv_zero_extend_32_to_64(bits) & 3 =="),
    ("Bitvector Refinements: Bitvector Constants", "src/list.rs", "#[constant(bv_int_to_bv32(7))]", "#[constant(bv_int_to_bv32(3))]"),
    ("`char` Literals", "src/list.rs", "['0' <= c && c <= '9']", "['0' <= c && c <= '2']"),
    ("`String` Literals", "src/list.rs", 'if status == Status::Open { "[ ] " }', 'if status == Status::Open { "[x] " }'),
    ("Extern Specs: Extern Functions", "src/list.rs", "-> T{v: <T as Ord>::max_res(a, b, v)}", "-> T"),
    ("Extern Specs: Options", "src/list.rs", "Option<u32[digit_value(c)]>", "Option<u32>"),
    ("Extern Specs: Vec", "src/list.rs", "removed.push(self.items.remove(index));", "self.items.remove(index);"),
    ("Extern Specs: Extern Structs", "src/list.rs", "    #[refined_by(elems: Set<T>)]\n    struct BTreeSet", "    #[refined_by(size: int)]\n    struct BTreeSet"),
    ("Extern Specs: Extern Impls", "src/list.rs", "T[@value]) -> bool[!set_is_in(value, s)]", "T[@value]) -> bool[set_is_in(value, s)]"),
    ("Extern Specs: Extern Traits", "src/list.rs", "#[spec(fn() -> Self{value: <Self as Default>::is_default(value)})]", "#[spec(fn() -> Self)]"),
    ("`for` loops with range `i..j`", "src/list.rs", "for i in 0..self.len() {\n            visit(i);", "for i in 0..=self.len() {\n            visit(i);"),
    ("Associated Refinements: Basic", "src/list.rs", "{ set_is_in(index, picks.picked) })]", "{ !set_is_in(index, picks.picked) })]"),
    ("Associated Refinements: Check Subtyping at Impl", "src/list.rs", "    #[spec(fn(&StatusSet[@set], Status[@status]) -> bool[set.bits & bit_of(status) != 0])]\n    fn has", "    #[spec(fn(&StatusSet[@set], Status[@status]) -> bool)]\n    fn has"),
    ("Associated Refinements: Associated Types", "src/list.rs", "{ set.bits & bit_of(status) != 0 })]\nimpl Selection for StatusSet", "{ set.bits & bit_of(status) == 0 })]\nimpl Selection for StatusSet"),
    ("Associated Refinements: Default", "src/list.rs", "#[assoc(fn need(selection: Self) -> int { 0 })]", "#[assoc(fn need(selection: Self) -> int { 1 })]"),
    ("Associated Refinements: Use in Extern Spec", "src/list.rs", "{ index < list.len })]", "{ index <= list.len })]"),
    ("Checking Overflows", "Cargo.toml", 'check_overflow = "strict"', 'check_overflow = "none"'),
    ("Extensible Properties for Primitive Ops", "src/list.rs", "(x == 1 && y == 2 => [<<](x, y) == 4)", "(x == 1 && y == 2 => [<<](x, y) == 8)"),
    ("Casting Sorts", "src/list.rs", "cast(c) - cast('0')", "cast(c) - cast('1')"),
    ("Detached Specifications", "src/list.rs", "-> bool[a == b];", "-> bool[a != b];"),
    ("Include Patterns", "Cargo.toml", 'include_trusted = ["src/rvec.rs"]\n', ""),
    ("Scraping Qualifiers", "src/main.rs", '#[opts(scrape_quals = "true")]\n', ""),
    ("Invariant Macro", "src/main.rs", "        invariant!(width: int, rest: int; width <= total - rest + 1);\n", ""),
    ("Qualifier Macro", "src/list.rs", "            qualifier!(left: int, taken: int, all: int; left + taken == all);\n", ""),
    ("Uninterpreted Operators", "src/rejected.rs", '#[opts(uif_ops = "*,/")]\n', ""),
]


def flux() -> tuple[bool, str]:
    """Whether Flux accepts the program, and how many functions it ignored."""
    out = subprocess.run(["./verify.sh"], capture_output=True, text=True)
    # One summary per crate; this crate's comes last.
    ignored = re.findall(r"(\d+) ignored", out.stdout + out.stderr)
    return out.returncode == 0, ignored[-1] if ignored else ""


def main() -> int:
    wanted = sys.argv[1] if len(sys.argv) > 1 else ""
    accepted, ignored = flux()
    if not accepted:
        print("the program does not verify as it is; fix that first")
        return 1
    survived = 0
    for section, file, old, new, *how in MUTANTS:
        if wanted not in section:
            continue
        path = pathlib.Path(file)
        original = path.read_text()
        if old not in original:
            print(f"GONE      {section}: the line to break is no longer in {file}")
            survived += 1
            continue
        path.write_text(original.replace(old, new, 1))
        try:
            now_accepted, now_ignored = flux()
        finally:
            path.write_text(original)
        caught = now_ignored != ignored if how == [UNCOUNTED] else not now_accepted
        print(f"{'caught  ' if caught else 'SURVIVED'}  {section}")
        survived += not caught
    print(f"{survived} not caught")
    return 1 if survived else 0


if __name__ == "__main__":
    sys.exit(main())
