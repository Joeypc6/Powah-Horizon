fn kit_guide_key_powah(game: &crate::game::Game<'_>, screen: &crate::game::native_hooks::NativeScreenView<'_>, key: &str, repeat: bool, canceled: &mut bool) {
    powah_optional::if_kit!("powah", {
    if game.player.is_none() {
        return;
    }
    crate::game::hz_kit_guide_powah::key(game, screen.menu, key, repeat, canceled);
} else { let _ = (game, screen, key, repeat, canceled); () })
}
