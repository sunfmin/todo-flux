use std::{
    env, fs,
    path::PathBuf,
    process::{Command, Output, Stdio},
};

struct Cli {
    file: PathBuf,
}

impl Cli {
    fn new(name: &str) -> Cli {
        let file = env::temp_dir().join(format!("todo-test-{}-{name}.txt", std::process::id()));
        let _ = fs::remove_file(&file);
        Cli { file }
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_todo"))
            .args(args)
            .env("TODO_FILE", &self.file)
            .output()
            .unwrap()
    }

    fn ok(&self, args: &[&str]) -> String {
        let out = self.run(args);
        assert!(
            out.status.success(),
            "{args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap()
    }

    fn stored(&self) -> String {
        fs::read_to_string(&self.file).unwrap()
    }
}

impl Drop for Cli {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.file);
    }
}

#[test]
fn add_done_undo_rm_round_trip() {
    let cli = Cli::new("round-trip");
    assert_eq!(cli.ok(&[]), "nothing to do\n");

    cli.ok(&["add", "buy", "milk"]);
    cli.ok(&["add", "write report"]);
    assert_eq!(
        cli.ok(&["list"]),
        " 1. [ ] !2 buy milk\n 2. [ ] !2 write report\n0/2 done (0%), 2 left, 0 doing\n  0 open at !1\n  2 open at !2\n  0 open at !3\n"
    );

    assert_eq!(
        cli.ok(&["done", "2"]),
        " 1. [ ] !2 buy milk\n 2. [x] !2 write report\n1/2 done (50%), 1 left, 0 doing\n  0 open at !1\n  1 open at !2\n  0 open at !3\n"
    );
    assert_eq!(cli.stored(), "[ ] !2 buy milk\n[x] !2 write report\n");

    cli.ok(&["undo", "2"]);
    assert_eq!(cli.stored(), "[ ] !2 buy milk\n[ ] !2 write report\n");

    assert_eq!(
        cli.ok(&["rm", "1"]),
        "removed: buy milk\n 1. [ ] !2 write report\n0/1 done (0%), 1 left, 0 doing\n  0 open at !1\n  1 open at !2\n  0 open at !3\n"
    );
    assert_eq!(cli.stored(), "[ ] !2 write report\n");

    assert_eq!(
        cli.ok(&["rm", "1"]),
        "removed: write report\nnothing to do\n"
    );
    assert_eq!(cli.stored(), "");
}

#[test]
fn status_follows_the_rules() {
    let cli = Cli::new("status");
    cli.ok(&["add", "a"]);

    cli.ok(&["start", "1"]);
    assert_eq!(cli.stored(), "[~] !2 a\n");
    cli.ok(&["start", "1"]);
    cli.ok(&["undo", "1"]);
    assert_eq!(cli.stored(), "[ ] !2 a\n");
    cli.ok(&["start", "1"]);
    cli.ok(&["done", "1"]);
    cli.ok(&["done", "1"]);
    assert_eq!(cli.stored(), "[x] !2 a\n");

    let out = cli.run(&["start", "1"]);
    assert!(!out.status.success());
    assert_eq!(
        String::from_utf8_lossy(&out.stderr),
        "todo: 'a' is done and cannot become doing; undo it first\n"
    );
    assert_eq!(cli.stored(), "[x] !2 a\n");

    cli.ok(&["undo", "1"]);
    cli.ok(&["start", "1"]);
    assert!(
        cli.ok(&["list"])
            .starts_with(" 1. [~] !2 a\n0/1 done (0%), 1 left, 1 doing\n")
    );
}

#[test]
fn list_can_show_one_status_under_the_original_numbers() {
    let cli = Cli::new("filter");
    for title in ["a", "b", "c"] {
        cli.ok(&["add", title]);
    }
    cli.ok(&["start", "2"]);
    cli.ok(&["done", "3"]);

    let summary =
        "1/3 done (33%), 2 left, 1 doing\n  0 open at !1\n  2 open at !2\n  0 open at !3\n";
    assert_eq!(
        cli.ok(&["list", "open"]),
        format!(" 1. [ ] !2 a\n{summary}")
    );
    assert_eq!(
        cli.ok(&["list", "doing"]),
        format!(" 2. [~] !2 b\n{summary}")
    );
    assert_eq!(
        cli.ok(&["list", "done"]),
        format!(" 3. [x] !2 c\n{summary}")
    );
    assert_eq!(
        cli.ok(&["list", "done", "open", "open"]),
        format!(" 1. [ ] !2 a\n 3. [x] !2 c\n{summary}")
    );
    assert!(!cli.run(&["list", "later"]).status.success());
    assert!(!cli.run(&["list", "open", "later"]).status.success());
}

#[test]
fn now_is_the_first_todo_being_worked_on() {
    let cli = Cli::new("now");
    assert_eq!(cli.ok(&["now"]), "nothing in progress\n");
    for title in ["a", "b", "c"] {
        cli.ok(&["add", title]);
    }
    assert_eq!(cli.ok(&["now"]), "nothing in progress\n");

    cli.ok(&["start", "3"]);
    cli.ok(&["start", "2"]);
    assert_eq!(cli.ok(&["now"]), " 2. [~] !2 b\n");

    cli.ok(&["done", "2"]);
    assert_eq!(cli.ok(&["now"]), " 3. [~] !2 c\n");
}

#[test]
fn next_is_the_first_open_todo_of_the_most_urgent_priority() {
    let cli = Cli::new("next");
    assert_eq!(cli.ok(&["next"]), "nothing to do\n");
    for title in ["a", "b", "c", "d"] {
        cli.ok(&["add", title]);
    }
    assert_eq!(cli.ok(&["next"]), " 1. [ ] !2 a\n");

    cli.ok(&["pri", "1", "3"]);
    cli.ok(&["pri", "3", "1"]);
    cli.ok(&["pri", "4", "1"]);
    assert_eq!(cli.ok(&["next"]), " 3. [ ] !1 c\n");

    cli.ok(&["done", "3"]);
    cli.ok(&["start", "4"]);
    assert_eq!(cli.ok(&["next"]), " 4. [~] !1 d\n");

    for n in ["1", "2", "4"] {
        cli.ok(&["done", n]);
    }
    assert_eq!(cli.ok(&["next"]), "nothing to do\n");
}

#[test]
fn priority_is_set_and_kept_in_range() {
    let cli = Cli::new("priority");
    cli.ok(&["add", "a"]);
    cli.ok(&["pri", "1", "1"]);
    assert_eq!(cli.stored(), "[ ] !1 a\n");
    cli.ok(&["pri", "1", "3"]);
    assert_eq!(cli.stored(), "[ ] !3 a\n");

    cli.ok(&["add", "b"]);
    cli.ok(&["done", "2"]);
    assert!(
        cli.ok(&["pri", "2", "1"])
            .ends_with("  0 open at !1\n  0 open at !2\n  1 open at !3\n")
    );
    cli.ok(&["rm", "2"]);

    for level in ["0", "4", "-1", "high", "300"] {
        assert!(!cli.run(&["pri", "1", level]).status.success(), "{level}");
    }
    assert!(!cli.run(&["pri", "2", "1"]).status.success());
    assert_eq!(cli.stored(), "[ ] !3 a\n");
}

#[test]
fn mv_reorders_without_losing_todos() {
    let cli = Cli::new("mv");
    for title in ["a", "b", "c"] {
        cli.ok(&["add", title]);
    }
    cli.ok(&["mv", "1", "3"]);
    assert_eq!(cli.stored(), "[ ] !2 b\n[ ] !2 c\n[ ] !2 a\n");
    cli.ok(&["mv", "3", "1"]);
    assert_eq!(cli.stored(), "[ ] !2 a\n[ ] !2 b\n[ ] !2 c\n");
    cli.ok(&["mv", "2", "2"]);
    assert_eq!(cli.stored(), "[ ] !2 a\n[ ] !2 b\n[ ] !2 c\n");

    for args in [["mv", "0", "1"], ["mv", "1", "4"], ["mv", "4", "1"]] {
        assert!(!cli.run(&args).status.success(), "{args:?}");
    }
    assert_eq!(cli.stored(), "[ ] !2 a\n[ ] !2 b\n[ ] !2 c\n");
}

#[test]
fn swap_exchanges_two_todos_and_leaves_the_rest() {
    let cli = Cli::new("swap");
    for title in ["a", "b", "c", "d"] {
        cli.ok(&["add", title]);
    }
    cli.ok(&["swap", "1", "3"]);
    assert_eq!(cli.stored(), "[ ] !2 c\n[ ] !2 b\n[ ] !2 a\n[ ] !2 d\n");
    cli.ok(&["swap", "4", "2"]);
    assert_eq!(cli.stored(), "[ ] !2 c\n[ ] !2 d\n[ ] !2 a\n[ ] !2 b\n");
    cli.ok(&["swap", "3", "4"]);
    assert_eq!(cli.stored(), "[ ] !2 c\n[ ] !2 d\n[ ] !2 b\n[ ] !2 a\n");
    cli.ok(&["swap", "2", "2"]);
    assert_eq!(cli.stored(), "[ ] !2 c\n[ ] !2 d\n[ ] !2 b\n[ ] !2 a\n");

    for args in [
        ["swap", "0", "1"],
        ["swap", "1", "5"],
        ["swap", "5", "1"],
        ["swap", "x", "1"],
    ] {
        assert!(!cli.run(&args).status.success(), "{args:?}");
    }
    assert!(!cli.run(&["swap", "1"]).status.success());
    assert_eq!(cli.stored(), "[ ] !2 c\n[ ] !2 d\n[ ] !2 b\n[ ] !2 a\n");
}

#[test]
fn rm_takes_several_numbers_but_none_twice() {
    let cli = Cli::new("rm-several");
    for title in ["a", "b", "c", "d"] {
        cli.ok(&["add", title]);
    }
    assert!(
        cli.ok(&["rm", "4", "1", "3"])
            .starts_with("removed: a\nremoved: c\nremoved: d\n 1. [ ] !2 b\n")
    );
    assert_eq!(cli.stored(), "[ ] !2 b\n");

    cli.ok(&["add", "e"]);
    for args in [&["rm", "1", "1"][..], &["rm", "1", "3"], &["rm"]] {
        assert!(!cli.run(args).status.success(), "{args:?}");
    }
    let out = cli.run(&["rm", "2", "1", "2"]);
    assert_eq!(
        String::from_utf8_lossy(&out.stderr),
        "todo: todo 2 is named twice\n"
    );
    assert_eq!(cli.stored(), "[ ] !2 b\n[ ] !2 e\n");
}

#[test]
fn clear_removes_every_done_todo() {
    let cli = Cli::new("clear");
    for title in ["a", "b", "c"] {
        cli.ok(&["add", title]);
    }
    cli.ok(&["done", "1"]);
    cli.ok(&["start", "2"]);
    cli.ok(&["done", "3"]);
    assert!(
        cli.ok(&["clear"])
            .starts_with("removed: a\nremoved: c\n 1. [~] !2 b\n")
    );
    assert_eq!(cli.stored(), "[~] !2 b\n");

    assert!(cli.ok(&["clear"]).starts_with(" 1. [~] !2 b\n"));
    assert!(!cli.run(&["clear", "1"]).status.success());
}

#[test]
fn count_prints_how_many_todos_are_in_each_status() {
    let cli = Cli::new("count");
    assert_eq!(cli.ok(&["count"]), "0 open, 0 doing, 0 done\n");

    fs::write(
        &cli.file,
        "[ ] !2 a\n[x] !2 b\n[~] !1 c\n[x] !3 d\n[ ] !2 e\n[x] !2 f\n[x] !2 g\n",
    )
    .unwrap();
    let stored = cli.stored();
    assert_eq!(cli.ok(&["count"]), "2 open, 1 doing, 4 done\n");
    assert_eq!(cli.stored(), stored);

    let out = cli.run(&["count", "open"]);
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    assert!(String::from_utf8_lossy(&out.stderr).starts_with("todo: cannot run 'count open'\n"));
}

#[test]
fn numbers_line_up_in_a_long_list() {
    let cli = Cli::new("long");
    fs::write(&cli.file, "[ ] !2 t\n".repeat(100)).unwrap();
    let out = cli.ok(&["list"]);
    assert!(out.starts_with("  1. [ ] !2 t\n  2. [ ] !2 t\n"));
    assert!(out.contains("\n 99. [ ] !2 t\n100. [ ] !2 t\n"));
    assert_eq!(cli.ok(&["next"]), "  1. [ ] !2 t\n");
}

#[test]
fn rm_takes_ranges() {
    let cli = Cli::new("rm-range");
    for title in ["a", "b", "c", "d", "e", "f"] {
        cli.ok(&["add", title]);
    }
    assert!(cli.ok(&["rm", "2-4", "6"]).starts_with(
        "removed: b\nremoved: c\nremoved: d\nremoved: f\n 1. [ ] !2 a\n 2. [ ] !2 e\n"
    ));
    assert!(cli.ok(&["rm", "2-2"]).starts_with("removed: e\n"));
    assert_eq!(cli.stored(), "[ ] !2 a\n");

    for title in ["b", "c"] {
        cli.ok(&["add", title]);
    }
    for args in [
        ["rm", "3-1", "1"],
        ["rm", "1-4", "1"],
        ["rm", "0-2", "1"],
        ["rm", "1-", "1"],
        ["rm", "-2", "1"],
        ["rm", "1-2-3", "1"],
    ] {
        assert!(!cli.run(&args).status.success(), "{args:?}");
    }
    let out = cli.run(&["rm", "1-2", "2-3"]);
    assert_eq!(
        String::from_utf8_lossy(&out.stderr),
        "todo: todo 2 is named twice\n"
    );
    assert_eq!(cli.stored(), "[ ] !2 a\n[ ] !2 b\n[ ] !2 c\n");
}

#[test]
fn out_of_range_positions_are_rejected_without_changing_the_file() {
    let cli = Cli::new("out-of-range");
    cli.ok(&["add", "only one"]);

    for args in [
        ["done", "0"],
        ["done", "2"],
        ["rm", "2"],
        ["undo", "x"],
        ["rm", "-1"],
    ] {
        let out = cli.run(&args);
        assert!(!out.status.success(), "{args:?} should fail");
        assert!(out.stdout.is_empty());
    }
    assert_eq!(cli.stored(), "[ ] !2 only one\n");
}

#[test]
fn bad_commands_fail() {
    let cli = Cli::new("bad-commands");
    assert!(!cli.run(&["add"]).status.success());
    assert!(!cli.run(&["add", "  "]).status.success());
    assert!(!cli.run(&["frobnicate"]).status.success());
    assert!(!cli.run(&["done"]).status.success());
    assert!(!cli.run(&["done", "1", "2"]).status.success());
    assert!(!cli.run(&["mv", "1"]).status.success());
    assert!(!cli.file.exists());
}

#[test]
fn hand_edited_file_loads() {
    let cli = Cli::new("hand-edited");
    fs::write(
        &cli.file,
        "no mark\n\n[x] finished\n[ ] !1 urgent\n[x] !9 bad priority\n!3 low\n[ ] !2  \n[~]   padded  \n[ ] !02 two digits\n",
    )
    .unwrap();
    cli.ok(&["mv", "1", "1"]);
    assert_eq!(
        cli.stored(),
        "[ ] !2 no mark\n[x] !2 finished\n[ ] !1 urgent\n[x] !2 !9 bad priority\n[ ] !3 low\n[~] !2 padded\n[ ] !2 !02 two digits\n"
    );
}

#[test]
fn multi_line_title_stays_one_todo() {
    let cli = Cli::new("multi-line");
    cli.ok(&["add", "line one\nline two"]);
    assert_eq!(cli.stored(), "[ ] !2 line one line two\n");
}

#[test]
fn title_that_looks_like_a_priority_survives_a_round_trip() {
    let cli = Cli::new("tag-title");
    cli.ok(&["add", "!1 not a priority"]);
    cli.ok(&["done", "1"]);
    assert_eq!(cli.stored(), "[x] !2 !1 not a priority\n");
}

#[test]
fn reader_that_stops_early_is_not_an_error() {
    let cli = Cli::new("closed-pipe");
    // More than a pipe holds, so the write cannot finish before the reader goes.
    fs::write(&cli.file, "[ ] !2 some todo\n".repeat(20_000)).unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_todo"))
        .arg("list")
        .env("TODO_FILE", &cli.file)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    let out = child.wait_with_output().unwrap();

    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stderr.is_empty());
}
