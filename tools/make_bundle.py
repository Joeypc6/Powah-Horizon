"""Writes the Powah bundle's horizon-mixin.toml and its one-line hook helpers, applies the kit and packs the content.

Usage: python make_bundle.py
"""

import json
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
BUNDLE = ROOT / "mixins" / "powah"
BUILD = ROOT.parent / "native-build" / "main" / "powah"
JAR = ROOT.parent / "Powah-1.16.5-2.3.16.jar"
HORIZON = "fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca"
MIXINS = "5a64af938ec86b00d53d16a408aabc0038fb3e57"
VERSION = "2.3.16"
M = "crate::game::powah"

GAME_MUT = ("game", "&mut crate::game::Game<'_>", "&mut *self")
GAME = ("game", "&crate::game::Game<'_>", "&*self")
ENTITY = ("entity", "&mut horizon_client::containers::ModBlockEntity", "&mut *entity")

# (id, hook, bindings, body)
HOOKS = [
    ("interact-entity", "interact_entity", [GAME_MUT, ("entity", "i32", "entity"), ("kind", "&str", "kind"), ("stack", "Option<&horizon_client::items::ItemStack>", "stack"), ("claimed", "&mut bool", "&mut *claimed")], "interact_entity(game, entity, kind, stack, claimed)"),
    ("be", "block_entity", [("block", "&str", "block"), ("entity", "&mut Option<(String, horizon_world::nbt::Compound)>", "&mut *entity")], "block_entity(block, entity)"),
    ("be-ticks", "block_entity_ticks", [("block", "&str", "block"), ("ticks", "&mut bool", "&mut *ticks")], "block_entity_ticks(block, ticks)"),
    ("be-container", "block_entity_container", [("block", "&str", "block"), ("container", "&mut Option<horizon_client::containers::ModContainer>", "&mut *container")], "block_entity_container(block, container)"),
    ("be-energy", "block_entity_energy", [("block", "&str", "block"), ("energy", "&mut Option<horizon_client::containers::ModEnergyBuffer>", "&mut *energy")], "block_entity_energy(block, energy)"),
    ("be-tick", "block_entity_tick", [GAME_MUT, ("pos", "[i32; 3]", "pos"), ENTITY], "block_entity_tick(game, pos, entity)"),
    ("be-fluids", "block_entity_fluids", [("block", "&str", "block"), ("fluids", "&mut Option<horizon_client::containers::ModFluidTanks>", "&mut *fluids")], "block_entity_fluids(block, fluids)"),
    ("accepts-fluid", "target:horizon_client::game::native_hooks::Game<'_>::native_block_entity_accepts_fluid", [("block", "&str", "block"), ("tank", "usize", "tank"), ("fluid", "&str", "fluid"), ("cancel", "&mut bool", "&mut *cancel")], "accepts_fluid(block, tank, fluid, cancel)"),
    ("item-energy", "item_energy_buffer", [("item", "&str", "item"), ("buffer", "&mut Option<crate::game::native_item_energy::ItemEnergyBuffer>", "&mut *buffer")], "item_energy_buffer(item, buffer)"),
    ("accepts", "target:horizon_client::game::native_hooks::Game<'_>::native_container_accepts", [GAME, ("entity", "&horizon_client::containers::ModBlockEntity", "&*entity"), ("slot", "usize", "slot"), ("stack", "&horizon_client::items::ItemStack", "stack"), ("face", "Option<horizon_core::geometry::Direction>", "face"), ("cancel", "&mut bool", "&mut *cancel")], "container_accepts(game, entity, slot, stack, face, cancel)"),
    ("slot-rules", "menu_slot_rules", [("menu", "&str", "menu"), ("slot", "usize", "slot"), ("stack", "&ItemStack", "stack"), ("may_place", "&mut bool", "&mut *may_place"), ("max_stack", "&mut u32", "&mut *max_stack")], None),
    ("menu-button", "menu_button_v2", [("pos", "[i32; 3]", "pos"), ENTITY, ("id", "i32", "id")], "menu_button(pos, entity, id)"),
    ("right-click", "right_click_block", [GAME_MUT, ("pos", "[i32; 3]", "pos"), ("canceled", "&mut bool", "&mut *canceled")], "right_click(game, pos, canceled)"),
    ("placed", "place_block", [GAME_MUT, ("stack", "&ItemStack", "stack"), ("pos", "[i32; 3]", "pos")], "placed(game, stack, pos)"),
    ("neighbor", "neighbor_changed", [("level", "&GameLevel<'_>", "&*self"), ("pos", "BlockPos", "pos"), ("state", "BlockStateId", "state")], "neighbor_changed(&level.facts.get(state).name, [pos.x, pos.y, pos.z])"),
    ("placement", "target:horizon_client::game::native_hooks::GameLevel<'_>::native_state_for_placement", [("level", "&crate::game::GameLevel<'_>", "&*self"), ("state", "horizon_world::registry::BlockStateId", "state"), ("context", "&horizon_world::level::PlacementContext", "context"), ("answer", "&mut Option<Option<horizon_world::registry::BlockStateId>>", "&mut *answer")], "state_for_placement(level, state, context, answer)"),
    ("screen", "screen", [("block", "&str", "block"), ("screen", "&mut Option<crate::game::native_hooks::NativeScreen>", "&mut *screen")], "screen(block, screen)"),
    ("screen-draw", "screen_draw", [GAME, ("block", "&str", "block"), ("data", "&[i32]", "data"), ("batch", "&mut horizon_client::hud::HudBatch", "&mut *batch"), ("origin", "[f32; 2]", "origin")], "screen_draw(game, block, data, batch, origin)"),
    ("screen-click", "screen_click", [GAME, ("block", "&str", "block"), ("data", "&[i32]", "data"), ("at", "[f32; 2]", "at"), ("right", "bool", "right"), ("button", "&mut Option<i32>", "&mut *button")], "screen_click(block, data, at, right, game.has_shift_down(), button)"),
    ("screen-tooltip", "screen_tooltip", [("block", "&str", "block"), ("data", "&[i32]", "data"), ("at", "[f32; 2]", "at"), ("lines", "&mut Vec<String>", "&mut *lines")], "screen_tooltip(block, data, at, lines)"),
    ("use-item", "use_item", [GAME_MUT, ("stack", "&ItemStack", "stack"), ("claimed", "&mut bool", "&mut *claimed")], "use_item(game, stack, claimed)"),
    ("tooltip", "tooltip", [("stack", "&ItemStack", "stack"), ("lines", "&mut Vec<(Text, u32)>", "&mut *lines")], "tooltip(stack, lines)"),
    ("world-saved", "world_saved", [GAME_MUT], "world_saved(game)"),
    ("item-bar", "item_bar", [("stack", "&ItemStack", "stack"), ("bar", "&mut Option<(i32, u32)>", "&mut *bar")], "item_bar(stack, bar)"),
    ("inventory-tick", "inventory_tick", [GAME_MUT, ("slot", "i32", "slot"), ("selected", "bool", "selected")], "inventory_tick(game, slot, selected)"),
    ("destroy", "destroy_block", [GAME_MUT, ("pos", "[i32; 3]", "pos")], "destroyed(game, pos)"),
    ("level-tick", "level_tick_post", [GAME_MUT, ("dimension", "&str", "dimension")], "level_tick(game, dimension)"),
    ("submit", "block_entity_submit", [GAME, ("pos", "[i32; 3]", "pos"), ("block", "&str", "block"), ("state", "horizon_world::registry::BlockStateId", "state"), ("entity", "&horizon_client::containers::ModBlockEntity", "&*entity"), ("draw", "&mut crate::game::native_block_entity_draw::BlockEntityDraw", "&mut *draw")], "submit(game, pos, block, state, entity, draw)"),
    ("render-bounds", "block_entity_render_bounding_box", [("block", "&str", "block"), ("entity", "&horizon_client::containers::ModBlockEntity", "&*entity"), ("bounds", "&mut [f32; 6]", "&mut *bounds")], "render_bounds(block, entity, bounds)"),
    ("update-tag", "block_entity_update_tag", [("entity", "&horizon_client::containers::ModBlockEntity", "&*entity"), ("tag", "&mut Option<horizon_world::nbt::Compound>", "&mut *tag")], "update_tag(entity, tag)"),
    ("layers", "target:horizon_client::game::native_hooks::Game<'_>::native_layer_definitions", [("layers", "&mut Vec<(String, horizon_client::mobs::model_layers::LayerDefinition)>", "&mut *layers")], "layer_definitions(layers)"),
    ("kit-frame", "frame", [GAME_MUT], "kit_frame(game)"),
]

# The charging slots' rule, in the client library (no `crate::game` there): a copy of powah.rs's is_chargeable.
SLOT_RULES = """fn powah_slot_rules(menu: &str, slot: usize, stack: &ItemStack, may_place: &mut bool, max_stack: &mut u32) {
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
"""

SPECIAL_HELPERS = {
    "embedded_content.rs": """// Powah is one file on the registry: its blocks, recipes and textures (and Lollipop's GUI textures) are the content
// mod in this bundle's data/ (made by tools/make_content.py from the jar). It is written out once and loaded as a dev mod.
fn powah_embedded_content(root: &std::path::Path, enabled: &std::collections::BTreeSet<String>, dev: &[std::path::PathBuf]) -> core::ops::ControlFlow<std::path::PathBuf> {
    static CONTENT: &[u8] = horizon_client::native_data!("powah", "powah-content.hmod");
    let dir = std::env::current_dir().unwrap_or_default().join(".horizon-cache");
    let file = dir.join("powah-embedded-content.hmod");
    if dev.contains(&file) {
        return core::ops::ControlFlow::Continue(());
    }
    let same = std::fs::read(&file).is_ok_and(|on_disk| on_disk == CONTENT);
    if !same && std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(&file, CONTENT)).is_err() {
        log::warn!("Horizon: Powah's content could not be written to {}", file.display());
        return core::ops::ControlFlow::Continue(());
    }
    let mut with = dev.to_vec();
    with.push(file);
    core::ops::ControlFlow::Break(content_root(root, enabled, &with))
}
""",
    "list.rs": """// The mod's row in Options > Mods, and its one key.
fn powah_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    mods.push(crate::game::native_hooks::NativeMod {
        id: "powah".into(),
        title: "Powah 2.3.16".into(),
        description: "Energy cells, cables, generators, a reactor and energizing.".into(),
        keys: vec![["guide".to_string(), "Open Guide for Items".to_string(), "key.keyboard.w".to_string()]],
    });
}
""",
    "drops.rs": """fn powah_drops(game: &mut crate::game::Game<'_>, pos: [i32; 3], state: BlockStateId, explosion: Option<f32>, drops: &mut Vec<ItemStack>) {
    let block = game.res.facts().get(state).name.to_string();
    crate::game::powah::block_drops(game, pos, &block, explosion.is_some(), drops);
}
""",
}


def binding_lines(bindings):
    return "".join(f'  {{ name = "{n}", ty = "{t}", value = "{v}" }},\n' for n, t, v in bindings)


def main():
    BUNDLE.mkdir(parents=True, exist_ok=True)
    out = [
        "[mod]",
        'id = "powah"',
        f'version = "{VERSION}"',
        "",
        "# The Horizon commit this bundle was built and tested on.",
        "[compatibility]",
        f'horizon_revision = "{HORIZON}"',
        "",
        "[[module]]",
        'id = "powah"',
        'file = "crates/horizon-client/src/game.rs"',
        'name = "powah"',
        'source = "powah.rs"',
        "",
        "[[module]]",
        'id = "powah-slots"',
        'file = "crates/horizon-client/src/game/machines/native_items.rs"',
        'name = "powah_slots"',
        'source = "slots.rs"',
        "",
    ]
    for hook_id, hook, bindings, body in HOOKS:
        helper = f"h_{hook_id.replace('-', '_')}.rs"
        args = ", ".join(f"{n}: {t}" for n, t, _ in bindings)
        if body is None:
            text = SLOT_RULES
        else:
            text = f"fn powah_{hook_id.replace('-', '_')}({args}) {{\n    {M}::{body};\n}}\n"
        (BUNDLE / helper).write_text(text, encoding="utf8")
        target = [f'target = "{hook[7:]}"'] if hook.startswith("target:") else [f'hook = "{hook}"', "hook_version = 1"]
        out += ["[[injection]]", f'id = "powah-{hook_id}"', 'kind = "HEAD"', *target, f'helper = "{helper}"', "bindings = [", binding_lines(bindings).rstrip("\n"), "]", ""]
    for name, text in SPECIAL_HELPERS.items():
        (BUNDLE / name).write_text(text, encoding="utf8")
    out += ["[[injection]]", 'id = "powah-list"', 'kind = "HEAD"', 'hook = "mods"', "hook_version = 1", 'helper = "list.rs"', "bindings = [", binding_lines([("mods", "&mut Vec<crate::game::native_hooks::NativeMod>", "&mut *mods")]).rstrip("\n"), "]", ""]
    out += ["[[injection]]", 'id = "powah-drops"', 'kind = "HEAD"', 'target = "horizon_client::game::native_hooks::Game<\'_>::native_block_drops"', 'helper = "drops.rs"', "bindings = [", binding_lines([GAME_MUT, ("pos", "[i32; 3]", "pos"), ("state", "BlockStateId", "state"), ("explosion", "Option<f32>", "explosion"), ("drops", "&mut Vec<ItemStack>", "&mut *drops")]).rstrip("\n"), "]", ""]
    out += ["[[injection]]", 'id = "powah-embedded-content"', 'kind = "HEAD"', "cancellable = true", 'target = "horizon_client::game::mods::content_root"', 'helper = "embedded_content.rs"', "bindings = [", binding_lines([("root", "&std::path::Path", "root"), ("enabled", "&std::collections::BTreeSet<String>", "enabled"), ("dev", "&[std::path::PathBuf]", "dev")]).rstrip("\n"), "]", ""]
    (BUNDLE / "horizon-mixin.toml").write_text("\n".join(out), encoding="utf8")
    # The JEI window, Mob Drops, World Gen, the creative panel and the W guide: this mod's own copy.
    subprocess.run([sys.executable, str(ROOT.parent / "jei-kit" / "apply.py"), "powah", str(BUNDLE)], check=True)
    for fixer in ("unused_helper_params.py", "use_helper_params.py"):
        subprocess.run([sys.executable, str(ROOT.parent / "tools" / fixer), str(BUNDLE)], check=True)
    (BUNDLE / "data").mkdir(exist_ok=True)
    subprocess.run([sys.executable, str(ROOT / "tools" / "make_content.py"), str(JAR), str(BUNDLE / "data" / "powah-content.hmod")], check=True)
    # The dev build folder: pinned, with no hooks number, so the pin applies.
    target = BUILD / "mixins" / "powah"
    target.mkdir(parents=True, exist_ok=True)
    for old in target.rglob("*"):
        if old.is_file():
            old.unlink()
    for f in BUNDLE.rglob("*"):
        if f.is_file():
            to = target / f.relative_to(BUNDLE)
            to.parent.mkdir(parents=True, exist_ok=True)
            to.write_bytes(f.read_bytes())
    (BUILD / "horizon-native.json").write_text(json.dumps({"game": "26.3", "horizon": HORIZON, "id": "powah", "mixins": MIXINS, "schema": 1, "version": VERSION}, indent=2), encoding="utf8")
    print("bundle written:", len(HOOKS) + 3, "injections")


if __name__ == "__main__":
    main()
