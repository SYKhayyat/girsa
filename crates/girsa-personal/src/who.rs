//! Who is writing.
//!
//! Every correction and every note carries a name, because a patch file handed
//! to somebody else has to say where it came from (spec.md §11). There is no
//! account and no registry — this is a name on a line in your own file, and the
//! machine's idea of who is sitting at it is the best guess available.
//!
//! # Why it is here and not at each place that needs one
//!
//! It was in two places, and they disagreed. `girsa-notes` read `GIRSA_WHO`
//! first; the window read `USERNAME` first and had never heard of `GIRSA_WHO`.
//! So a reader who set the variable — the only way this project offers to say
//! *call me something else* — got the name they chose on notes written from the
//! terminal and their operating-system login on every patch made in the
//! application, in the same personal layer, on the same afternoon.
//!
//! Nothing announced it. Two files, four lines, and one of them silently
//! outranked the setting whose whole job was to outrank the other.

/// The name to stamp on what you write.
///
/// `GIRSA_WHO` first, because it is the only one you chose. Then the operating
/// system's login — `USERNAME` on Windows, `USER` everywhere else — and then
/// `me`, which is a true statement about a personal layer and does not pretend
/// to be a name.
#[must_use]
pub fn who() -> String {
    from(|name| std::env::var(name).ok())
}

/// The order, without the environment — which is what a test can hold still.
///
/// Reading `std::env` in a test sets it for every other test in the process,
/// and the failure that causes shows up in whichever test happens to run
/// second.
fn from(look: impl Fn(&str) -> Option<String>) -> String {
    for name in NAMES {
        if let Some(said) = look(name) {
            let said = said.trim();
            // An empty variable is not an answer. `USER=` is set on some
            // service accounts, and a patch signed with the empty string
            // reads as a patch nobody wrote.
            if !said.is_empty() {
                return said.to_string();
            }
        }
    }
    NOBODY.to_string()
}

/// Where a name is looked for, in order.
const NAMES: &[&str] = &["GIRSA_WHO", "USERNAME", "USER"];

/// What a personal layer says when nobody said anything.
const NOBODY: &str = "me";

#[cfg(test)]
mod tests {
    use super::*;

    fn saying(pairs: &[(&str, &str)]) -> String {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        from(|name| {
            pairs
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.clone())
        })
    }

    #[test]
    fn the_one_you_chose_outranks_the_one_the_machine_knows() {
        // The disagreement this module exists to end: the window read
        // `USERNAME` first, so setting `GIRSA_WHO` changed the name on your
        // notes and not the name on your corrections.
        assert_eq!(
            saying(&[("GIRSA_WHO", "שאול"), ("USERNAME", "Administrator")]),
            "שאול"
        );
    }

    #[test]
    fn windows_and_everywhere_else_are_both_asked() {
        assert_eq!(saying(&[("USERNAME", "Administrator")]), "Administrator");
        assert_eq!(saying(&[("USER", "shaul")]), "shaul");
    }

    #[test]
    fn an_empty_variable_is_not_a_name() {
        // `USER=` is set on some service accounts. A patch signed with the
        // empty string reads as a patch nobody wrote, which is worse than one
        // signed `me`.
        assert_eq!(saying(&[("GIRSA_WHO", ""), ("USER", "shaul")]), "shaul");
        assert_eq!(saying(&[("USER", "   ")]), NOBODY);
    }

    #[test]
    fn nobody_at_all_is_still_a_true_statement() {
        assert_eq!(saying(&[]), "me");
    }

    #[test]
    fn a_name_is_trimmed() {
        assert_eq!(saying(&[("GIRSA_WHO", " שאול ")]), "שאול");
    }
}
