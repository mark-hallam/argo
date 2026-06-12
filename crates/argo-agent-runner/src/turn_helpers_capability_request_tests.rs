use super::classify_access;

#[test]
fn read_leaning_verbs_classify_as_read() {
    for tool in [
        "salesforce_get_account",
        "list_contacts",
        "search_tickets",
        "fetch_invoice",
        "gmail_read",
        "query_db",
        "show_record",
        "describe_table",
        "lookup_user",
    ] {
        assert_eq!(classify_access(tool), "read", "{tool} should be read");
    }
}

#[test]
fn mutating_or_unknown_tools_classify_as_write() {
    for tool in [
        "salesforce_create_lead",
        "send_email",
        "delete_record",
        "wire_funds",
        "post_message",
        "stripe",
    ] {
        assert_eq!(classify_access(tool), "write", "{tool} should be write");
    }
}

#[test]
fn classification_is_case_insensitive() {
    assert_eq!(classify_access("GET_Account"), "read");
    assert_eq!(classify_access("SEND_Email"), "write");
}
