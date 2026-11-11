"""Powah's W guide, from its own book (owmii.powah.client.book.PowahBook, Lollipop's Wiki): the same categories and
entries in the same order, each entry's text from the jar's lang (`wiki.powah.<entry>_<n>`), with 3D scenes that
turn with the mouse showing the blocks joined as they work: a generator cabled to a cell, the energizing orb with its
rods on cables, the reactor's 3x4x3 multiblock, and an ender cell with its gate."""
import json
import re

import guide_index

ID = "powah"
TIERS = ["starter", "basic", "hardened", "blazing", "niotic", "spirited", "nitro"]
# PowahBook.WIKI: category → its entries.
BOOK = {
    "generators": ["furnator", "magmator", "thermo_generator", "solar_panel", "reactor"],
    "storage_transfer": ["energy_cell", "energy_cable", "ender_cell", "ender_gate"],
    "energy_blocks": ["energizing", "player_transmitter", "energy_hopper", "energy_discharger"],
    "items": ["battery", "wrench", "charged_snowball", "player_aerial_pearl", "binding_card", "binding_card_dim", "lens_of_ender"],
    "materials": ["uraninite", "dry_ice"],
}
# The items an entry's page is for (W over any of them opens it).
TIERED = {"furnator", "magmator", "thermo_generator", "solar_panel", "reactor", "energy_cell", "energy_cable",
          "ender_cell", "ender_gate", "player_transmitter", "energy_hopper", "energy_discharger", "battery"}
ITEMS = {
    "energizing": ["energizing_orb"] + [f"energizing_rod_{t}" for t in TIERS],
    "energy_cell": [f"energy_cell_{t}" for t in TIERS + ["creative"]],
    "uraninite": ["uraninite", "uraninite_raw", "uraninite_raw_poor", "uraninite_raw_dense", "uraninite_ore",
                  "uraninite_ore_poor", "uraninite_ore_dense", "uraninite_block"],
}


def text(lang, key):
    """A lang line with `<powah:x>` item links as the guide's item tags, and %s left as the book fills it."""
    s = lang.get(key, "")
    # EnergizingConfig.range (default 4), which the book passes through Lollipop's Text.toRange.
    s = s.replace("%s", "4")
    return re.sub(r"<([a-z0-9_]+:[a-z0-9_/]+)>", lambda m: f'<ItemLink id="{m.group(1)}" />', s)


def items_for(entry):
    if entry in ITEMS:
        return [f"{ID}:{i}" for i in ITEMS[entry]]
    if entry in TIERED:
        return [f"{ID}:{entry}_{t}" for t in TIERS]
    return [f"{ID}:{entry}"]


def scenes(files):
    north = {"facing": "north"}
    s = {}
    # A furnator feeding a cell through a cable: the cable joins both (its arms east and west).
    s["furnator"] = guide_index.scene(files, ID, "generator_to_cell", [
        (0, 0, 0, f"{ID}:furnator_basic", north),
        (1, 0, 0, f"{ID}:energy_cable_basic", {"east": "true", "west": "true"}),
        (2, 0, 0, f"{ID}:energy_cell_basic")])
    s["energy_cable"] = s["furnator"]
    # The orb, and rods round it standing on cables, each pointing at the orb (EnergizingRodBlock's facing).
    orb = [(2, 1, 2, f"{ID}:energizing_orb", {"facing": "down"})]
    for (x, z, facing) in [(0, 2, "east"), (4, 2, "west"), (2, 0, "south"), (2, 4, "north")]:
        orb.append((x, 0, z, f"{ID}:energy_cable_basic", {"up": "true"}))
        orb.append((x, 1, z, f"{ID}:energizing_rod_basic", {"facing": "down"}))
    s["energizing"] = guide_index.scene(files, ID, "energizing", orb)
    # Builder.getPosList: the reactor's 3x4x3 around the block placed (its bottom middle).
    reactor = [(x, y, z, f"{ID}:reactor_basic") for x in range(3) for y in range(4) for z in range(3)]
    s["reactor"] = guide_index.scene(files, ID, "reactor", reactor)
    # An ender cell and a gate on another cell's face: both draw from the owner's ender network.
    s["ender_cell"] = guide_index.scene(files, ID, "ender_network", [
        (0, 0, 0, f"{ID}:ender_cell_basic"), (3, 0, 0, f"{ID}:energy_cell_basic"),
        (3, 1, 0, f"{ID}:ender_gate_basic", {"facing": "down"})])
    s["ender_gate"] = s["ender_cell"]
    s["solar_panel"] = guide_index.scene(files, ID, "solar", [
        (x, 0, 0, f"{ID}:solar_panel_basic") for x in range(3)] + [(1, -1, 0, f"{ID}:energy_cable_basic", {"up": "true"})])
    return s


def write(files, lang):
    pages = {}
    shown = scenes(files)
    welcome = "\n\n".join(text(lang, f"wiki.powah.welcome_back_{n}") for n in range(8) if lang.get(f"wiki.powah.welcome_back_{n}"))
    pages["powah.md"] = ("---\nnavigation:\n  title: Powah\nitem_ids:\n- powah:book\n---\n\n# Powah\n\n"
                         + (welcome or "Generate, store and move Forge Energy (FE).") + "\n\n"
                         + "\n".join(f"- [{lang.get(f'wiki.powah.{c}', c)}]({c}.md)" for c in BOOK) + "\n")
    for category, entries in BOOK.items():
        title = lang.get(f"wiki.powah.{category}", category)
        pages[f"{category}.md"] = (f"---\nnavigation:\n  title: {title}\nitem_ids: []\n---\n\n# {title}\n\n"
                                   + "\n".join(f"- [{lang.get(f'wiki.powah.{e}', e)}]({e}.md)" for e in entries) + "\n")
        for e in entries:
            name = lang.get(f"wiki.powah.{e}", e)
            body = "\n\n".join(text(lang, f"wiki.powah.{e}_{n}") for n in range(12) if lang.get(f"wiki.powah.{e}_{n}"))
            items = "\n".join(f"- {i}" for i in items_for(e))
            page = f"---\nnavigation:\n  title: {name}\nitem_ids:\n{items}\n---\n\n# {name}\n\n{body}\n"
            if e in shown:
                page += "\n" + shown[e]
            pages[f"{e}.md"] = page
    # Working with the other ports: each part and page shows only when that mod is installed too (tested in game:
    # a Powah cell fills an AE2 energy acceptor, a Hostile Neural Networks chamber and an Iron Furnaces furnace).
    others = [
        ("with_ae2", "With AE2", "ae2:energy_acceptor",
         "Cables and cells feed an AE2 **Energy Acceptor**, which turns FE into the network's AE.",
         [(0, 0, 0, f"{ID}:energy_cell_basic"), (1, 0, 0, f"{ID}:energy_cable_basic", {"east": "true", "west": "true"}),
          (2, 0, 0, "ae2:energy_acceptor"), (3, 0, 0, "ae2:fluix_glass_cable"), (4, 0, 0, "ae2:controller")]),
        ("with_hostilenetworks", "With Hostile Neural Networks", "hostilenetworks:sim_chamber",
         "A cell or cable on a **Simulation Chamber** or **Loot Fabricator** powers it.",
         [(0, 0, 0, f"{ID}:energy_cell_basic"), (1, 0, 0, "hostilenetworks:sim_chamber", {"facing": "north"}),
          (2, 0, 0, f"{ID}:energy_cable_basic", {"east": "true", "west": "true"}), (3, 0, 0, "hostilenetworks:loot_fabricator", {"facing": "north"})]),
        ("with_ironfurnaces", "With Iron Furnaces", "ironfurnaces:augment_factory",
         "A furnace with a **Factory** augment runs on power: a cell beside it fills it.",
         [(0, 0, 0, f"{ID}:energy_cell_basic"), (1, 0, 0, "ironfurnaces:diamond_furnace", {"facing": "north"})]),
    ]
    links = ""
    for name, title, item, body, blocks in others:
        pages[f"{name}.md"] = (f"---\nnavigation:\n  title: {title}\nitem_ids: []\nrequires:\n- {item}\n---\n\n# {title}\n\n"
                               + body + "\n\n" + guide_index.scene(files, ID, name, blocks))
        links += guide_index.if_mods([item], f"- [{title}]({name}.md)\n")
    pages["powah.md"] += "\n" + links
    guide_index.write(files, ID, pages)
    return len(pages)


if __name__ == "__main__":
    import sys
    import zipfile
    lang = json.loads(zipfile.ZipFile(sys.argv[1]).read("assets/powah/lang/en_us.json").decode("utf-8-sig"))
    files = {}
    print(write(files, lang), "pages,", len(files), "files")
