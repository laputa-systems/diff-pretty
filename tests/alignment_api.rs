use diff_pretty::align::Alignment;

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
