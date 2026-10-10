use std::{
    env, fs,
    path::PathBuf,
    process::{Command, Output},
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
        " 1. [ ] !2 buy milk\n 2. [ ] !2 write report\n0/2 done (0%), 2 left\n  0 open at !1\n  2 open at !2\n  0 open at !3\n"
    );

    assert_eq!(
        cli.ok(&["done", "2"]),
        " 1. [ ] !2 buy milk\n 2. [x] !2 write report\n1/2 done (50%), 1 left\n  0 open at !1\n  1 open at !2\n  0 open at !3\n"
    );
    assert_eq!(cli.stored(), "[ ] !2 buy milk\n[x] !2 write report\n");

    cli.ok(&["undo", "2"]);
    assert_eq!(cli.stored(), "[ ] !2 buy milk\n[ ] !2 write report\n");

    assert_eq!(
        cli.ok(&["rm", "1"]),
        "removed: buy milk\n 1. [ ] !2 write report\n0/1 done (0%), 1 left\n  0 open at !1\n  1 open at !2\n  0 open at !3\n"
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
            .starts_with(" 1. [~] !2 a\n0/1 done (0%), 1 left\n")
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

    let summary = "1/3 done (33%), 2 left\n  0 open at !1\n  2 open at !2\n  0 open at !3\n";
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
    assert!(!cli.run(&["list", "later"]).status.success());
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
        "no mark\n\n[x] finished\n[ ] !1 urgent\n[x] !9 bad priority\n!3 low\n[ ] !2  \n[~]   padded  \n",
    )
    .unwrap();
    cli.ok(&["mv", "1", "1"]);
    assert_eq!(
        cli.stored(),
        "[ ] !2 no mark\n[x] !2 finished\n[ ] !1 urgent\n[x] !2 !9 bad priority\n[ ] !3 low\n[~] !2 padded\n"
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
