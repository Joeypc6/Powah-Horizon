fn kit_guide_frame_powah(game: &mut crate::game::Game<'_>) {
    powah_optional::if_kit!("powah", {
    if game.player.is_some() {
        crate::game::hz_kit_guide_powah::frame_hook(game);
    }
} else { let _ = game; () })
}
