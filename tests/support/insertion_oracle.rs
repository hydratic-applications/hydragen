//! Fixture-owned permitted insertions, independent of the production editors.

/// Check exact text at known source offsets and remove only those insertions.
/// Offsets and inserted text must come from the fixture, never the candidate.
pub fn preserves(source: &str, candidate: &str, insertions: &[(usize, &str)]) -> bool {
    let mut previous = 0;
    let mut output = 0;
    let mut reconstructed = Vec::with_capacity(source.len());
    for &(position, addition) in insertions {
        let Some(untouched) = source.as_bytes().get(previous..position) else {
            return false;
        };
        let Some(retained) = candidate.as_bytes().get(output..output + untouched.len()) else {
            return false;
        };
        if retained != untouched {
            return false;
        }
        reconstructed.extend_from_slice(retained);
        output += untouched.len();
        if candidate.as_bytes().get(output..output + addition.len()) != Some(addition.as_bytes()) {
            return false;
        }
        output += addition.len();
        previous = position;
    }
    let suffix = &source.as_bytes()[previous..];
    let Some(retained) = candidate.as_bytes().get(output..) else {
        return false;
    };
    if retained != suffix {
        return false;
    }
    reconstructed.extend_from_slice(retained);
    reconstructed == source.as_bytes()
}

pub fn assert_insertions(source: &str, candidate: &str, insertions: &[(usize, &str)]) {
    assert!(
        preserves(source, candidate, insertions),
        "edit differs from fixture-permitted insertions {insertions:?}:\n{source:?}\n=>\n{candidate:?}"
    );
}
