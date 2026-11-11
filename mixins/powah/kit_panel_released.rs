fn jei_window_released(screen: &crate::game::native_hooks::NativeScreenView<'_>) {
    powah_optional::if_kit_jei!("powah", {
    crate::game::hz_kit_window_powah::release(screen.mouse);
} else { let _ = screen; () })
}
