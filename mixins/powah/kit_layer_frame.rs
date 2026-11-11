// `frame`: the layer's work while JEI is on; its window shut when JEI is turned off.
fn kit_layer_frame_powah(game: &mut crate::game::Game<'_>) {
    powah_optional::if_kit_jei!("powah", {
    if game.native_on(crate::game::jei::ID) {
        crate::game::hz_kit_layer_powah::frame(game);
    } else if crate::game::hz_kit_window_powah::is_open() {
        // JEI turned off with its window open: nothing would close it, and it would keep taking the screen's input.
        crate::game::hz_kit_window_powah::close();
        let _ = crate::game::hz_kit_window_powah::take_acts();
    }
} else { let _ = game; () })
}
