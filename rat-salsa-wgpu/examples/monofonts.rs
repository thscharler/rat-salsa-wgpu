pub fn main() {
    let mut font_db = fontdb::Database::new();
    font_db.load_system_fonts();

    let mut names = font_db
        .faces()
        .filter_map(|info| {
            if info.monospaced {
                if let Some((family, _)) = info.families.first() {
                    Some(family.clone())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();

    for name in names {
        println!("{:?},", name);
    }
}
