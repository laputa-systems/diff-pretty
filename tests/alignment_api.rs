use diff_pretty::align::Alignment;
use diff_pretty::align::Operation;

#[test]
fn empty_alignment_has_no_operations() {
    let alignment = Alignment::new(Vec::new(), Vec::new());
    let mut operations = vec![(Operation::Insertion, 9)];
    alignment.coalesced_operations_into(&mut operations);
    assert!(operations.is_empty(), "{operations:?}");
}

#[test]
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

#[test]
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
