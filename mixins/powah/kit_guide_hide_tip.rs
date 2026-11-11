// `native_tooltip_render_pre`: while the guide is open, the screen behind it shows no tooltip over its pages (the
// guide draws its own items' tooltips itself).
fn kit_guide_hide_tip_powah(cancel: &mut bool) {
    powah_optional::if_kit!("powah", {
    if crate::game::hz_kit_guide_powah::is_open() {
        *cancel = true;
    }
} else { let _ = cancel; () })
}
