use crow_mir::Substs;

pub fn mangle_name(name: &str, substs: &Substs) -> String {
    if substs.is_empty() { return name.to_string(); }
    let suffix: Vec<String> = substs.iter()
        .map(|t| sanitize(&format!("{t}")))
        .collect();
    format!("{name}__{}", suffix.join("_"))
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => c,
            _ => '_',
        })
        .collect()
}