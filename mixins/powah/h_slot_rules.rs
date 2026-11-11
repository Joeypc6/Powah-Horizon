fn powah_slot_rules(menu: &str, slot: usize, stack: &ItemStack, may_place: &mut bool, max_stack: &mut u32) {
    // canInsert and getSlotLimit: charging slots take what holds energy, one each; the furnator's fuel slot (1)
    // takes up to 64 of anything (the fuel check is the server's, by burn time).
    let n = stack.item.strip_prefix("powah:").unwrap_or("");
    let holds_energy = n.starts_with("battery_") || (n.starts_with("energy_cell_") && n != "energy_cell_creative");
    match menu {
        "powah:furnator" if slot == 1 => {}
        "powah:energy_hopper" => *may_place = false,
        "powah:ender_cell" => {
            *may_place = if slot == 0 { (n.starts_with("energy_cell_") && n != "energy_cell_creative") || n.starts_with("battery_") } else { holds_energy };
            *max_stack = 1;
        }
        "powah:reactor" => {
            *may_place = match slot {
                1 => n == "uraninite",
                3 => stack.item.as_ref() == "minecraft:redstone" || stack.item.as_ref() == "minecraft:redstone_block",
                4 => matches!(stack.item.as_ref(), "minecraft:snow_block" | "minecraft:snowball" | "minecraft:ice" | "minecraft:packed_ice" | "minecraft:blue_ice" | "powah:dry_ice"),
                0 => holds_energy,
                _ => true,
            };
        }
        // canInsert: a card bound to a player (bound_player_id), one.
        "powah:player_transmitter" => {
            *may_place = (n == "binding_card" || n == "binding_card_dim")
                && stack.extra.as_ref().is_some_and(|e| e.other.iter().any(|(k, v)| k == "minecraft:custom_data" && format!("{v:?}").contains("bound_player_id")));
            *max_stack = 1;
        }
        "powah:energy_cell" | "powah:furnator" | "powah:magmator" | "powah:thermo_generator" | "powah:solar_panel" | "powah:energy_discharger" => {
            *may_place = holds_energy;
            if menu != "powah:furnator" {
                *max_stack = 1;
            }
        }
        _ => {}
    }
}
