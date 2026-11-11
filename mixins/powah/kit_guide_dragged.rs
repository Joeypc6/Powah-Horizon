fn kit_guide_dragged_powah(delta: [f32; 2], canceled: &mut bool) {
    powah_optional::if_kit!("powah", {
    crate::game::hz_kit_guide_powah::drag(delta, canceled);
} else { let _ = (delta, canceled); () })
}
