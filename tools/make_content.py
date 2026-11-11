"""Powah's blocks, items, recipes and textures as a Horizon content mod, from the 1.16.5 jar.

Usage: python make_content.py <Powah jar> <out.hmod>

The jar's assets go in as they are (block states, models, textures, lang, particles). On top, what 26.3 and Horizon need:
- an item definition (`assets/powah/items/<id>.json`) for every item model;
- each block's declaration: its template, strength (Blcks: metal 2/20, the ores rock), its state properties as its block
  state file names them, and the shapes from its class (cables, gates, rods, the orb, panels, transmitters);
- each item's declaration with its stack size (Itms);
- the recipes in 26.3's form (an ingredient is an id or a #tag, a result an `id`), Forge's tags as `c:` ones;
- loot tables with Horizon's `type` key for a condition;
- the creative tab and the guide's pages.
The energizing recipes stay `powah:energizing`: the native mod reads them.
"""

import io
import json
import re
import sys
import zipfile

ID = "powah"
# Lollipop 3.2.9's source (owmii/Lollipop, branch 1.16, commit a7ff674), which Powah needs; its assets ship here.
LOLLIPOP = "Z:/AE2 Converter/deps/lollipop/Lollipop-1.16-a7ff674.zip"
TIERS = ["starter", "basic", "hardened", "blazing", "niotic", "spirited", "nitro"]
MACHINES = ["energy_cell", "ender_cell", "energy_cable", "ender_gate", "energizing_rod", "furnator", "magmator",
            "thermo_generator", "solar_panel", "reactor", "player_transmitter", "energy_hopper", "energy_discharger"]
PLAIN = {
    # Properties.metal(2, 20) / rock(h, 8) (Lollipop: iron / stone sounds; rock needs a pickaxe).
    "energized_steel_block": (2.0, 20.0, "minecraft:iron_block"),
    "blazing_crystal_block": (2.0, 20.0, "minecraft:iron_block"),
    "niotic_crystal_block": (2.0, 20.0, "minecraft:iron_block"),
    "spirited_crystal_block": (2.0, 20.0, "minecraft:iron_block"),
    "nitro_crystal_block": (2.0, 20.0, "minecraft:iron_block"),
    "uraninite_block": (2.0, 20.0, "minecraft:iron_block"),
    "uraninite_ore_poor": (3.0, 8.0, "minecraft:stone"),
    "uraninite_ore": (3.2, 8.0, "minecraft:stone"),
    "uraninite_ore_dense": (4.0, 8.0, "minecraft:stone"),
    "dry_ice": (2.0, 8.0, "minecraft:stone"),
}
# Itms: stacksTo(1) / (16); the rest 64.
STACK = {"book": 1, "wrench": 1, "binding_card": 1, "binding_card_dim": 1, "charged_snowball": 16,
         **{f"battery_{t}": 1 for t in TIERS}}
# Forge's tags as NeoForge's `c:` ones, with vanilla's members where Horizon has no such tag.
C_TAGS = {
    "ingots/iron": ["minecraft:iron_ingot"],
    "ingots/gold": ["minecraft:gold_ingot"],
    "gems/diamond": ["minecraft:diamond"],
    "gems/emerald": ["minecraft:emerald"],
    "gems/quartz": ["minecraft:quartz"],
    "storage_blocks/quartz": ["minecraft:quartz_block"],
    "storage_blocks/redstone": ["minecraft:redstone_block"],
    "nether_stars": ["minecraft:nether_star"],
    "glass_blocks": ["minecraft:glass"],
    "glass_panes": ["minecraft:glass_pane"],
}
# Forge 1.16's tag names that 26.3's convention renamed.
RENAMED = {"forge:glass": "c:glass_blocks"}


MANIFEST = """# Powah's blocks and items, from Powah-1.16.5-2.3.16.jar by powah-horizon/tools/make_content.py.
id = "powah"
name = "Powah (blocks and items)"
version = "2.3.16"
description = "Powah's blocks, recipes and textures."
authors = ["owmii"]
api = "0.2"
side = "both"
capabilities = ["registries"]
# Lollipop's GUI textures, lang and empty model ship here under its own namespace.
namespaces = ["lollipop"]
"""


def px(*b):
    """A Block.box in pixels, as a sorted box in block units (func_208617_a sorts its corners)."""
    lo = [min(b[i], b[i + 3]) / 16 for i in range(3)]
    hi = [max(b[i], b[i + 3]) / 16 for i in range(3)]
    return [round(v, 5) for v in lo + hi]


FACES = ["up", "down", "north", "south", "west", "east"]

# EnergizingOrbBlock: the orb and its foot, by facing.
ORB = {
    "up": [px(3.5, 11.0, 3.5, 12.5, 1.77, 12.5), px(2.5, 15.0, 2.5, 13.5, 16.0, 13.5)],
    "down": [px(3.5, 14.23, 3.5, 12.5, 5.0, 12.5), px(2.5, 0.0, 2.5, 13.5, 1.0, 13.5)],
    "north": [px(3.5, 3.5, 14.23, 12.5, 12.5, 5.0), px(2.5, 2.5, 0.0, 13.5, 13.5, 1.0)],
    "south": [px(3.5, 3.5, 11.0, 12.5, 12.5, 1.77), px(2.5, 2.5, 15.0, 13.5, 13.5, 16.0)],
    "west": [px(14.23, 3.5, 3.5, 5.0, 12.5, 12.5), px(0.0, 2.5, 2.5, 1.0, 13.5, 13.5)],
    "east": [px(11.0, 3.5, 3.5, 1.77, 12.5, 12.5), px(15.0, 2.5, 2.5, 16.0, 13.5, 13.5)],
}
# EnergizingRodBlock: the tip, the base and the shaft between.
ROD = {
    "up": [px(7, 7, 7, 9, 9, 9), px(7, 13, 7, 9, 16, 9), px(7.25, 9, 7.25, 8.75, 13, 8.75)],
    "down": [px(7, 7, 7, 9, 9, 9), px(7, 0, 7, 9, 3, 9), px(7.25, 3, 7.25, 8.75, 7, 8.75)],
    "north": [px(7, 7, 7, 9, 9, 9), px(7, 7, 0, 9, 9, 3), px(7.25, 7.25, 3, 8.75, 8.75, 7)],
    "south": [px(7, 7, 7, 9, 9, 9), px(7, 7, 13, 9, 9, 16), px(7.25, 7.25, 13, 8.75, 8.75, 9)],
    "west": [px(7, 7, 7, 9, 9, 9), px(0, 7, 7, 3, 9, 9), px(3, 7.25, 7.25, 7, 8.75, 8.75)],
    "east": [px(7, 7, 7, 9, 9, 9), px(13, 7, 7, 16, 9, 9), px(13, 7.25, 7.25, 9, 8.75, 8.75)],
}
# EnderGateBlock: a thin plate on the face it stands on.
GATE = {
    "up": [px(6, 15.5, 6, 10, 16, 10)], "down": [px(6, 0, 6, 10, 0.5, 10)],
    "north": [px(6, 6, 0, 10, 10, 0.5)], "south": [px(6, 6, 15.5, 10, 10, 16)],
    "east": [px(15.5, 6, 6, 16, 10, 10)], "west": [px(0, 6, 6, 0.5, 10, 10)],
}
# CableBlock: the core and an arm to each joined side.
CABLE_CORE = px(6.25, 6.25, 6.25, 9.75, 9.75, 9.75)
CABLE_ARM = {
    "north": px(6.5, 6.5, 0.0, 9.5, 9.5, 7.0), "east": px(9.5, 6.5, 6.5, 16.0, 9.5, 9.5),
    "south": px(6.5, 6.5, 9.5, 9.5, 9.5, 16.0), "west": px(0.0, 6.5, 6.5, 6.5, 9.5, 9.5),
    "up": px(6.5, 9.5, 6.5, 9.5, 16.0, 9.5), "down": px(6.5, 0.0, 6.5, 9.5, 7.0, 9.5),
}
SOLAR = [px(0, 0, 0, 16, 1, 16)]
# PlayerTransmitterBlock: the bottom block's and the top's (TOP) shapes.
TRANSMITTER_BOTTOM = [px(6, 8, 6, 10, 12, 10), px(7, 4.5, 7, 9, 8, 9), px(4, 0.5, 4, 12, 1.5, 12),
                      px(4, 2, 4, 12, 3, 12), px(4, 3.5, 4, 12, 4.5, 12), px(1, -15, 1, 15, -1, 15), px(4, -1, 4, 12, 0, 12)]
TRANSMITTER_TOP = [px(6, 24, 6, 10, 28, 10), px(7, 20.5, 7, 9, 24, 9), px(4, 16.5, 4, 12, 17.5, 12),
                   px(4, 18, 4, 12, 19, 12), px(4, 19.5, 4, 12, 20.5, 12), px(1, 1, 1, 15, 15, 15), px(4, 15, 4, 12, 16, 12)]


def state_properties(blockstate):
    """The properties a block state file names, with their values in the order it first gives them."""
    props = {}

    def add(k, v):
        props.setdefault(k, [])
        if v not in props[k]:
            props[k].append(v)

    for key in blockstate.get("variants", {}):
        for part in filter(None, key.split(",")):
            k, v = part.split("=")
            add(k, v)
    for part in blockstate.get("multipart", []):
        when = part.get("when", {})
        for clause in when.get("OR", [when]):
            for k, v in clause.items():
                for one in str(v).lower().split("|"):
                    add(k, one)
    out = {}
    for k, values in props.items():
        if set(values) <= {"true", "false"}:
            out[k] = {"type": "boolean"}
        else:
            out[k] = {"type": "enum", "values": values}
    return out


def machine_kind(name):
    for m in MACHINES:
        if name.startswith(m + "_"):
            return m
    return None


def block_decl(name, blockstate):
    props = state_properties(blockstate)
    kind = machine_kind(name)
    if name in PLAIN:
        hardness, resistance, sound = PLAIN[name]
        decl = {"template": "minecraft:stone" if sound == "minecraft:stone" else "minecraft:iron_block",
                "properties": {"hardness": hardness, "explosion_resistance": resistance, "sounds": sound,
                               "requires_tool": True}}
        return decl
    # Properties.metalNoSolid(2, 20): an iron block's sound, no occlusion (the beacon's template occludes nothing), a
    # block entity of its own, as the other ports' machines.
    decl = {"template": "minecraft:beacon",
            "properties": {"hardness": 2.0, "explosion_resistance": 20.0, "sounds": "minecraft:iron_block",
                           "requires_tool": True}}
    if kind == "energy_cable":
        # The template that joins six ways; the native mod says which neighbours take energy.
        decl["template"] = "minecraft:chorus_plant"
        decl["connects_to"] = f"#{ID}:energy_cable_connects"
        props = {}
        sides = ["north", "east", "south", "west", "up", "down"]
        values = {}
        for mask in range(64):
            on = [(mask >> i) & 1 == 1 for i in range(6)]
            # chorus_plant's own properties sort: down, east, north, south, up, west.
            order = ["down", "east", "north", "south", "up", "west"]
            joined = dict(zip(sides, on))
            key = ",".join("true" if joined[s] else "false" for s in order)
            values[key] = [CABLE_CORE] + [CABLE_ARM[s] for s in sides if joined[s]]
        decl["shapes"] = {"properties": ["down", "east", "north", "south", "up", "west"], "values": values}
    elif kind in ("ender_gate", "energizing_rod") or name == "energizing_orb":
        table = {"ender_gate": GATE, "energizing_rod": ROD}.get(kind, ORB)
        decl["shapes"] = {"property": "facing", "values": {f: table[f] for f in FACES if f in props.get("facing", {}).get("values", FACES)}}
    elif kind == "solar_panel":
        decl["shapes"] = {"default": SOLAR}
    elif kind == "player_transmitter":
        decl["shapes"] = {"property": "top", "values": {"false": TRANSMITTER_BOTTOM, "true": TRANSMITTER_TOP}} if "top" in props else {"default": TRANSMITTER_BOTTOM}
    if kind == "reactor":
        props = dict(props, core={"type": "boolean"})
    if props:
        decl["state_properties"] = props
        # setStateProps: the orb, rods and gates stand on the face below them (DOWN), the machines face north.
        down = kind in ("ender_gate", "energizing_rod") or name == "energizing_orb"
        decl["default_state"] = {k: ("false" if v["type"] == "boolean" else
                                     ("down" if down else "north") if k == "facing" and "north" in v["values"] else v["values"][0])
                                 for k, v in props.items()}
    return decl


def ingredient(v):
    """1.16's {"item"} / {"tag"} (or a list of them) as 26.3's id / #tag."""
    if isinstance(v, list):
        return [ingredient(x) for x in v]
    if "item" in v:
        return v["item"]
    tag = RENAMED.get(v["tag"], v["tag"])
    return "#" + tag.replace("forge:", "c:")


def recipe(doc):
    kind = doc.get("type", "")
    out = dict(doc)
    if "key" in out:
        out["key"] = {k: ingredient(v) for k, v in out["key"].items()}
    if "ingredients" in out:
        out["ingredients"] = [ingredient(v) for v in out["ingredients"]]
    if "ingredient" in out:
        out["ingredient"] = ingredient(out["ingredient"])
    r = out.get("result")
    if isinstance(r, str):
        r = {"item": r}
    if isinstance(r, dict) and "item" in r:
        out["result"] = {"id": r["item"], **({"count": r["count"]} if r.get("count", 1) != 1 else {})}
    if kind in ("minecraft:crafting_shaped", "minecraft:crafting_shapeless"):
        out.setdefault("category", "misc")
    return out


def fix_conditions(value):
    if isinstance(value, dict):
        return {("type" if k == "condition" else k): fix_conditions(v) for k, v in value.items()}
    if isinstance(value, list):
        return [fix_conditions(v) for v in value]
    return value


def tag_values(doc):
    return [v.replace("forge:", "c:") if isinstance(v, str) else v for v in doc.get("values", [])]


def main(jar, out):
    source = zipfile.ZipFile(jar)
    files = {}
    blocks, items = [], []
    for name in source.namelist():
        if name.endswith("/"):
            continue
        data = source.read(name)
        if name.startswith(f"assets/{ID}/"):
            if name.startswith(f"assets/{ID}/blockstates/"):
                block = name.rsplit("/", 1)[1][:-5]
                blocks.append(block)
                bs = json.loads(data.decode("utf-8-sig"))
                files[f"data/{ID}/horizon/block/{block}.json"] = json.dumps(block_decl(block, bs), indent=1).encode()
                if machine_kind(block) == "reactor":
                    # ReactorBlock.CORE, a property Java adds in code: the core and its 35 parts share one model.
                    model = bs["variants"][""]
                    data = json.dumps({"variants": {"core=false": model, "core=true": model}}, indent=2).encode()
            if name.startswith(f"assets/{ID}/models/item/") and name.endswith(".json"):
                items.append(name.rsplit("/", 1)[1][:-5])
            files[name] = data
            # The renderers' textures (textures/model/tile) are drawn from the block atlas, which takes a content mod's
            # textures/block/ only: a copy goes there too, the original kept.
            if name.startswith(f"assets/{ID}/textures/model/tile/"):
                files[name.replace("/textures/model/tile/", "/textures/block/tile/")] = data
            continue
        if name.startswith(f"data/{ID}/recipes/"):
            doc = json.loads(data.decode("utf-8-sig"))
            files[name.replace("/recipes/", "/recipe/")] = json.dumps(recipe(doc), indent=1).encode()
            continue
        if name.startswith(f"data/{ID}/loot_tables/"):
            doc = fix_conditions(json.loads(data.decode("utf-8-sig")))
            files[name.replace("/loot_tables/", "/loot_table/")] = json.dumps(doc, indent=1).encode()
            continue
        if name.startswith("data/forge/tags/"):
            doc = json.loads(data.decode("utf-8-sig"))
            rel = name[len("data/forge/tags/"):]
            kind, rest = rel.split("/", 1)
            kind = {"blocks": "block", "items": "item"}[kind]
            files[f"data/c/tags/{kind}/{rest}"] = json.dumps({"replace": False, "values": tag_values(doc)}, indent=1).encode()
            continue
        if name.startswith("data/curios/"):
            continue  # Curios isn't on Horizon; the charm slot is the native mod's (batteries charge from the inventory).
        if name == "pack.mcmeta" or name.startswith("META-INF") or name.endswith(".class"):
            continue
        files[name] = data
    block_set = set(blocks)
    # A block item's model is its block's; the rest are items of their own.
    item_ids = sorted(set(items))
    for item in item_ids:
        files[f"assets/{ID}/items/{item}.json"] = json.dumps({"model": {"type": "minecraft:model", "model": f"{ID}:item/{item}"}}).encode()
        if item in block_set:
            continue
        decl = {"template": "minecraft:paper"}
        if STACK.get(item, 64) != 64:
            decl["max_stack_size"] = STACK[item]
        files[f"data/{ID}/horizon/item/{item}.json"] = json.dumps(decl, indent=1).encode()
    for tag, values in C_TAGS.items():
        files.setdefault(f"data/c/tags/item/{tag}.json", json.dumps({"replace": False, "values": values}, indent=1).encode())
    # What a cable joins besides cables: every Powah block that takes or gives energy (the native mod adds the
    # machines of other mods it can reach).
    joins = sorted(b for b in blocks if machine_kind(b) and machine_kind(b) not in ("energy_cable",)) + ["powah:energizing_orb"]
    files[f"data/{ID}/tags/block/energy_cable_connects.json"] = json.dumps(
        {"replace": False, "values": [b if ":" in b else f"{ID}:{b}" for b in joins]}, indent=1).encode()
    files[f"data/{ID}/horizon/creative_tab/main.json"] = json.dumps(
        {"title": "itemGroup.powah", "icon": f"{ID}:energy_cell_nitro", "items": [f"{ID}:{i}" for i in item_ids]}, indent=1).encode()
    # Lollipop's own assets, 1:1 (LGPL-3.0, like Powah): the GUI textures Powah's screens are drawn from
    # (container/*, the slot overlays, the side-config and redstone buttons), its lang (info.lollipop.*) and the empty
    # block model (the up and down facings of the furnator, magmator and discharger, which a placement never gives).
    with zipfile.ZipFile(LOLLIPOP) as lolli:
        for name in lolli.namelist():
            rel = name.split("/src/main/resources/", 1)[-1]
            if rel.startswith("assets/lollipop/") and not name.endswith("/"):
                files[rel] = lolli.read(name)
    # The W guide, from Powah's own book.
    import powah_guide
    powah_guide.write(files, json.loads(source.read(f"assets/{ID}/lang/en_us.json").decode("utf-8-sig")))
    # The menus the native mod opens (powah.rs): the energy cell's two charging slots and its 14 data slots.
    for menu, slots in (("energy_cell", 2), ("furnator", 2), ("magmator", 1), ("thermo_generator", 1), ("solar_panel", 1), ("energy_discharger", 7), ("energy_hopper", 1), ("player_transmitter", 1), ("ender_cell", 3), ("reactor", 5), ("energy_cable", 1)):
        files[f"data/{ID}/horizon/menu/{menu}.json"] = json.dumps({"slots": slots, "data_slots": 48}, indent=2).encode()
    files["horizon-mod.toml"] = MANIFEST.encode()
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w", zipfile.ZIP_DEFLATED) as z:
        for name in sorted(files):
            z.writestr(name, files[name])
    open(out, "wb").write(buf.getvalue())
    print(f"{len(files)} files, {len(blocks)} blocks, {len(item_ids)} items, {len(buf.getvalue())} bytes")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
