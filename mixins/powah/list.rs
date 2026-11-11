// The mod's row in Options > Mods, and its one key.
fn powah_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    mods.push(crate::game::native_hooks::NativeMod {
        id: "powah".into(),
        title: "Powah 2.3.16".into(),
        description: "Energy cells, cables, generators, a reactor and energizing.".into(),
        keys: vec![["guide".to_string(), "Open Guide for Items".to_string(), "key.keyboard.w".to_string()]],
    });
}
