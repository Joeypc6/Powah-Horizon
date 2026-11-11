# Powah for Horizon

An unofficial port of [Powah](https://github.com/owmii/Powah) 2.3.16 (Minecraft 1.16.5, by owmii, LGPL-3.0) and the
parts of its library [Lollipop](https://github.com/owmii/Lollipop) it uses (LGPL-3.0) to
[Horizon](https://github.com/horizon-rust/horizon) and Minecraft 26.x, as one native mod. The Java code was rewritten
in Rust for Horizon's hooks. Textures, models and language files are Powah's. This port is under the LGPL-3.0 as
well (LICENSE, with the GPL-3.0 it extends in COPYING.GPL-3.0).

- `mixins/powah`: the mod, and its content pack in `mixins/powah/data`.
- `tools/`: make the content from the original jar and write the bundle.

## What's there

- Energy cells, cables, generators (furnator, magmator, thermo, solar, reactor), the energizing orb and rods,
  ender cells and gates, the player transmitter, batteries, binding cards, the wrench and the lenses, with Powah's
  screens, recipes and JEI categories, the W guide with 3D scenes, and Jade.
- Built for Horizon fdc87ae (Nightly 2026-11-08); see `horizon-native.json`.

## Tested

In game on Horizon `fdc87ae` (with the other ports): generators, cells, cables and the wrench, the energizing orb and rods,
batteries charging items, binding cards, the photoelectric pane turning into a lens of ender, the screens and JEI cards (fluids drawn
as textures), and the W guide. The block and item models were compared with the 2.3.16 jar's: all 287 the same, none missing.
