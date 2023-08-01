/// Check if all keywords are contained in this vector of strings.
/// Used to check the output of EXPLAIN. We use this method instead of
/// snapshot testing because small details (like cost) can change from
/// run to run rendering the output unstable.
pub fn is_contained_in_lines(keywords: Vec<&str>, lines: Vec<String>) {
    let connected_lines = lines.join("\n");
    tracing::info!(
        "expected keywords: {:?}\nlines: {}",
        keywords,
        connected_lines,
    );
    assert!(keywords.iter().all(|&s| connected_lines.contains(s)));
}
