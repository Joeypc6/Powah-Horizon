fn powah_destroy(game: &mut crate::game::Game<'_>, pos: [i32; 3]) {
    crate::game::powah::destroyed(game, pos);
}
