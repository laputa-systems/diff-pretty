use diff_pretty::align::{Alignment, Operation};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

fn empty_alignment_has_no_operations() {
    let alignment = Alignment::new(Vec::new(), Vec::new());
    let mut operations = vec![(Operation::Insertion, 9)];
    alignment.coalesced_operations_into(&mut operations);
    assert!(operations.is_empty(), "{operations:?}");
}

fn one_sided_alignment_accounts_for_every_token() {
    for (left, right, expected) in [
        (vec!["a", "b", "c"], Vec::new(), vec![(Operation::Deletion, 3)]),
        (Vec::new(), vec!["a", "b", "c"], vec![(Operation::Insertion, 3)]),
    ] {
        let alignment = Alignment::new(left, right);
        let mut operations = Vec::new();
        alignment.coalesced_operations_into(&mut operations);
        assert_eq!(operations, expected);
    }
}

fn alignment_tokens_are_readable_after_reset() {
    let mut alignment = Alignment::new(vec!["old"], vec!["new"]);
    assert_eq!(alignment.x_tokens(), &["old"]);
    assert_eq!(alignment.y_tokens(), &["new"]);
    alignment.reset_lines("before", "after");
    assert_eq!(alignment.x_tokens(), &["", "before"]);
    assert_eq!(alignment.y_tokens(), &["", "after"]);
    let mut operations = Vec::new();
    alignment.coalesced_operations_into(&mut operations);
    assert!(!operations.is_empty());
}

fn minimum_cost(x: &[&str], y: &[&str], after_match: bool) -> usize {
    if x.is_empty() && y.is_empty() { return 0; }
    let mut best = usize::MAX;
    if !x.is_empty() {
        best = best.min(2 + usize::from(after_match) + minimum_cost(&x[1..], y, false));
    }
    if !y.is_empty() {
        best = best.min(2 + usize::from(after_match) + minimum_cost(x, &y[1..], false));
    }
    if !x.is_empty() && !y.is_empty() && x[0] == y[0] {
        best = best.min(minimum_cost(&x[1..], &y[1..], true));
    }
    best
}

fn check_path(x: &[&str], y: &[&str], operations: &[(Operation, usize)]) {
    let (mut i, mut j, mut cost, mut after_match) = (0, 0, 0, true);
    let mut previous = None;
    for &(op, count) in operations {
        assert!(count > 0 && previous != Some(op), "invalid runs: {operations:?}");
        previous = Some(op);
        for _ in 0..count {
            match op {
                Operation::NoOp => {
                    assert!(i < x.len() && j < y.len() && x[i] == y[j], "{x:?} {y:?} {operations:?}");
                    i += 1; j += 1; after_match = true;
                }
                Operation::Deletion => { i += 1; cost += 2 + usize::from(after_match); after_match = false; }
                Operation::Insertion => { j += 1; cost += 2 + usize::from(after_match); after_match = false; }
            }
        }
    }
    assert_eq!((i, j), (x.len(), y.len()), "{x:?} {y:?} {operations:?}");
    assert_eq!(cost, minimum_cost(x, y, true), "{x:?} {y:?} {operations:?}");
}

fn scenarios() {
    empty_alignment_has_no_operations();
    one_sided_alignment_accounts_for_every_token();
    alignment_tokens_are_readable_after_reset();
    for (x, y, expected) in [
        (vec!["a"], vec!["b"], vec![(Operation::Deletion, 1), (Operation::Insertion, 1)]),
        (vec!["a"], vec!["b", "b"], vec![(Operation::Deletion, 1), (Operation::Insertion, 2)]),
        (vec!["a"], vec!["b", "b", "b"], vec![(Operation::Deletion, 1), (Operation::Insertion, 3)]),
        (vec!["a", "a"], vec!["a"], vec![(Operation::NoOp, 1), (Operation::Deletion, 1)]),
        (vec!["a"], vec!["a", "a"], vec![(Operation::NoOp, 1), (Operation::Insertion, 1)]),
    ] {
        let mut out = Vec::new();
        Alignment::new(x, y).coalesced_operations_into(&mut out);
        assert_eq!(out, expected);
    }
    let mut sequences = vec![Vec::new()];
    for len in 1..=3 {
        for bits in 0..(1 << len) {
            sequences.push((0..len).map(|i| if bits & (1 << i) == 0 { "a" } else { "b" }).collect());
        }
    }
    let mut out = vec![(Operation::NoOp, 99)];
    for x in &sequences {
        for y in &sequences {
            Alignment::new(x.clone(), y.clone()).coalesced_operations_into(&mut out);
            check_path(x, y, &out);
        }
    }
    let mut alignment = Alignment::new(Vec::new(), Vec::new());
    for (x, y) in [("a b c d e", "a b"), ("x", ""), ("", "y z"), ("same", "same"), ("long line again", "different line") ] {
        alignment.reset_lines(x, y);
        alignment.coalesced_operations_into(&mut out);
        check_path(alignment.x_tokens(), alignment.y_tokens(), &out);
        let first = out.clone();
        alignment.coalesced_operations_into(&mut out);
        assert_eq!(out, first);
    }
}

struct Reap(Child);
impl Drop for Reap {
    fn drop(&mut self) { let _ = self.0.kill(); let _ = self.0.wait(); }
}

#[test]
fn public_alignment_scenarios() {
    if std::env::var_os("DIFF_PRETTY_ALIGNMENT_CHILD").is_some() {
        eprintln!("alignment child entered");
        scenarios();
        eprintln!("alignment child completed");
        return;
    }
    let mut child = Reap(Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "public_alignment_scenarios", "--nocapture"])
        .env("DIFF_PRETTY_ALIGNMENT_CHILD", "1")
        .stdout(Stdio::null()).stderr(Stdio::inherit()).spawn().unwrap());
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            assert!(status.success(), "alignment child failed: {status}");
            break;
        }
        assert!(Instant::now() < deadline, "alignment child timed out");
        std::thread::sleep(Duration::from_millis(10));
    }
}
