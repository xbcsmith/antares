# Testing Improvements

Notes and follow-up suggestions
- The tests currently scan source files via simple string matching. That's a brittle approach (it can break if function signatures or file structure change). Consider one of the following for greater robustness:
  - Make the tests look for the presence/absence of widget ID patterns across the entire SDK source folder rather than limiting it to `main.rs` (i.e., check all `src/*.rs` in the sdk/campaign_builder crate).
  - Use a lightweight Rust source parser (e.g., `syn`) in test code to locate the `pub fn show` method AST and inspect specific method bodies (safer but more complexity).
  - Add a unit test or a small compile-time test API to assert usage of `ComboBox::from_id_salt` vs `ComboBox::from_label` where widgets are created (this would avoid brittle string parsing).
- I ran `cargo clippy` as an additional check, which surfaced multiple clippy warnings in other files (unrelated to this change). I did not fix those because they're pre-existing and out of scope for this test-only update. We may want to schedule those for a separate cleanup PR to fully satisfy `-D warnings`.
- If you'd like, I can:
  - Make the test even more resilient (e.g., search all editor files for `from_id_salt` usage),
  - Add a test that ensures all `ComboBox` uses across the SDK use `from_id_salt` rather than any `from_label`,
  - Or convert string-check tests into AST-based checks for future-proofing.

If you want me to proceed with any of the follow-ups, tell me which option you prefer and I’ll implement it.