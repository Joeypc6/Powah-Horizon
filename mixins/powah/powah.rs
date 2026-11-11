// Powah 2.3.16 (owmii, LGPL-3.0) for Horizon, with the parts of Lollipop 3.2.9 (owmii, LGPL-3.0) it is built on:
// AbstractEnergyStorage, SideConfig, Transfer, Redstone, Energy and the container screens. From the jar's classes and
// Lollipop's source (owmii/Lollipop, branch 1.16, a7ff674).
//
// Every change happens on the server: the screen only asks (a menu button), the server checks the block and applies
// it. Energy is the block entity's int `energy` (Horizon's buffer, `native_block_entity_energy`); Lollipop keeps a
// long, and every default capacity fits an int.
use super::Game;
powah_optional::kit_use!(guide as kit_guide);
powah_optional::if_mod_items!("jei", {
powah_optional::kit_use!(layer as kit_layer);
powah_optional::kit_use!(window as kit_window);
});
use horizon_client::containers::{ModBlockEntity, ModContainer, ModEnergyBuffer};
use horizon_client::hud::{HudBatch, rgba};
use horizon_client::items::ItemStack;
use horizon_core::geometry::Direction;
use horizon_world::nbt::{Compound, JavaString, Tag};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

type Pos = [i32; 3];

pub(in crate::game) const NS: &str = "powah";

// ---- Tiers (owmii.powah.block.Tier)

const TIERS: [&str; 7] = ["starter", "basic", "hardened", "blazing", "niotic", "spirited", "nitro"];

/// The block's kind and tier: `powah:energy_cell_basic` is ("energy_cell", Some(1)); the creative cell's tier is None.
fn kind_tier(block: &str) -> Option<(&str, Option<usize>)> {
    let name = block.strip_prefix("powah:")?;
    if name == "energy_cell_creative" {
        return Some(("energy_cell", None));
    }
    let (kind, tier) = name.rsplit_once('_')?;
    Some((kind, Some(TIERS.iter().position(|t| *t == tier)?)))
}

fn is_cell(block: &str) -> bool {
    kind_tier(block).is_some_and(|(k, _)| k == "energy_cell")
}

/// The machines with a screen and side config (AbstractEnergyStorage with a container).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mach {
    Cell,
    Furnator,
    Magmator,
    Thermo,
    Solar,
    Discharger,
    Hopper,
    Transmitter,
    EnderCell,
    EnderGate,
    Reactor,
}

fn machine(block: &str) -> Option<Mach> {
    Some(match kind_tier(block)?.0 {
        "energy_cell" => Mach::Cell,
        "furnator" => Mach::Furnator,
        "magmator" => Mach::Magmator,
        "thermo_generator" => Mach::Thermo,
        "solar_panel" => Mach::Solar,
        "energy_discharger" => Mach::Discharger,
        "energy_hopper" => Mach::Hopper,
        "player_transmitter" => Mach::Transmitter,
        "ender_cell" => Mach::EnderCell,
        "ender_gate" => Mach::EnderGate,
        "reactor" => Mach::Reactor,
        _ => return None,
    })
}

impl Mach {
    /// The menu id (the pack's data/powah/horizon/menu/*.json).
    fn menu(self) -> &'static str {
        match self {
            Mach::Cell => "powah:energy_cell",
            Mach::Furnator => "powah:furnator",
            Mach::Magmator => "powah:magmator",
            Mach::Thermo => "powah:thermo_generator",
            Mach::Solar => "powah:solar_panel",
            Mach::Discharger => "powah:energy_discharger",
            Mach::Hopper => "powah:energy_hopper",
            Mach::Transmitter => "powah:player_transmitter",
            Mach::EnderCell | Mach::EnderGate => "powah:ender_cell",
            Mach::Reactor => "powah:reactor",
        }
    }
    /// Its own slots, as the container places them (SlotBase(inv, i, x, y)).
    fn slots(self) -> &'static [[f32; 2]] {
        match self {
            Mach::Cell => &[[4.0, 4.0], [4.0, 29.0]],
            // FurnatorContainer: 0 the charging slot at 4,54; 1 the fuel at 87,18.
            Mach::Furnator => &[[4.0, 54.0], [87.0, 18.0]],
            Mach::Magmator | Mach::Thermo => &[[4.0, 54.0]],
            // SolarContainer has no slot of its own drawn: its one charging slot sits off the screen.
            Mach::Solar => &[[-1000.0, -1000.0]],
            // DischargerContainer: SlotBase(inv, i, 5 + i * 25, 54) for its seven slots.
            Mach::Discharger => &[[5.0, 54.0], [30.0, 54.0], [55.0, 54.0], [80.0, 54.0], [105.0, 54.0], [130.0, 54.0], [155.0, 54.0]],
            // EnergyHopperTile has no slots; a menu needs one, kept off the screen and closed to items.
            Mach::Hopper => &[[-1000.0, -1000.0]],
            // PlayerTransmitterContainer: SlotBase(inv, 0, 4, 29), the binding card.
            Mach::Transmitter => &[[4.0, 29.0]],
            // EnderCellContainer: SlotBase(inv, 0, 0, 1000) (the extender, off the screen), 1 at 4,4 and 2 at 4,29.
            Mach::EnderCell | Mach::EnderGate => &[[0.0, 1000.0], [4.0, 4.0], [4.0, 29.0]],
            // ReactorContainer: 0 charging at 4,54; 1 uraninite at 73,29; 2 carbon at 31,6; 3 redstone at 31,52;
            // 4 solid coolant at 120,52.
            Mach::Reactor => &[[4.0, 54.0], [73.0, 29.0], [31.0, 6.0], [31.0, 52.0], [120.0, 52.0]],
        }
    }
    /// The screen's background (`Textures`): texture, size; and the player inventory's y.
    fn screen(self) -> (&'static str, f32, f32, f32) {
        match self {
            Mach::Cell => ("powah:gui/container/energy_cell", 176.0, 141.0, 59.0),
            Mach::Furnator => ("powah:gui/container/furnator", 176.0, 166.0, 84.0),
            Mach::Magmator => ("powah:gui/container/magmator", 176.0, 166.0, 84.0),
            Mach::Thermo => ("powah:gui/container/thermo", 176.0, 166.0, 84.0),
            Mach::Solar => ("powah:gui/container/wide_energy", 176.0, 141.0, 59.0),
            Mach::Discharger => ("powah:gui/container/discharger", 176.0, 166.0, 84.0),
            Mach::Hopper => ("powah:gui/container/wide_energy", 176.0, 141.0, 59.0),
            Mach::Transmitter => ("powah:gui/container/player_transmitter", 176.0, 141.0, 59.0),
            Mach::EnderCell | Mach::EnderGate => ("powah:gui/container/ender_cell", 176.0, 164.0, 82.0),
            Mach::Reactor => ("powah:gui/container/reactor", 176.0, 166.0, 84.0),
        }
    }
    /// The config file its numbers are in.
    fn file(self) -> &'static str {
        match self {
            Mach::Cell => "energy_cell",
            Mach::Furnator => "generators/furnator",
            Mach::Magmator => "generators/magmator",
            Mach::Thermo => "generators/thermo_gen",
            Mach::Solar => "generators/solar_panel",
            Mach::Discharger => "energy_discharger",
            Mach::Hopper => "energy_hopper",
            Mach::Transmitter => "player_transmitter",
            Mach::EnderCell => "ender_cell",
            Mach::EnderGate => "ender_gate",
            Mach::Reactor => "generators/reactor",
        }
    }
    fn generator(self) -> bool {
        !matches!(self, Mach::Cell | Mach::Discharger | Mach::Hopper | Mach::Transmitter | Mach::EnderCell | Mach::EnderGate)
    }
    /// getTransferType EXTRACT: it only gives energy by its sides.
    fn gives_only(self) -> bool {
        !matches!(self, Mach::Cell | Mach::Hopper | Mach::Transmitter | Mach::EnderCell | Mach::EnderGate)
    }
}

fn is_cable(block: &str) -> bool {
    kind_tier(block).is_some_and(|(k, _)| k == "energy_cable")
}

fn is_creative(block: &str) -> bool {
    block == "powah:energy_cell_creative"
}

// ---- Config (config/powah/energy/*.toml, ForgeConfigSpec's layout: [Energy_Capacity] starterCap = ...)

/// The defaults of each file, by key (`Section.key`), as Powah's config classes define them.
fn defaults(file: &str) -> Vec<(String, i64)> {
    let (caps, transfers, gens): (&[i64], &[i64], &[i64]) = match file {
        "energy_cell" | "items/battery" | "energy_discharger" => (
            &[100_000, 1_000_000, 4_000_000, 10_000_000, 25_000_000, 60_000_000, 140_000_000],
            &[200, 2_500, 8_000, 20_000, 50_000, 100_000, 400_000],
            &[],
        ),
        "energy_cable" => (&[0; 7], &[100, 1_400, 5_000, 18_000, 40_000, 90_000, 200_000], &[]),
        "ender_cell" => (&[0; 7], &[200, 2_500, 8_000, 20_000, 50_000, 100_000, 400_000], &[]),
        "ender_gate" => (&[0; 7], &[100, 1_400, 5_000, 18_000, 40_000, 90_000, 200_000], &[]),
        "energizing" => (
            &[10_000, 100_000, 250_000, 800_000, 1_500_000, 4_000_000, 10_000_000],
            &[10, 50, 120, 300, 700, 1_200, 3_000],
            &[],
        ),
        "energy_hopper" | "player_transmitter" => (
            &[100_000, 1_000_000, 3_000_000, 5_000_000, 10_000_000, 15_000_000, 40_000_000],
            &[500, 2_500, 8_000, 20_000, 50_000, 100_000, 400_000],
            &[],
        ),
        "generators/furnator" | "generators/magmator" => (
            &[10_000, 100_000, 500_000, 900_000, 2_000_000, 5_000_000, 10_000_000],
            &[80, 200, 700, 1_200, 5_000, 7_000, 10_000],
            &[15, 70, 180, 400, 900, 2_000, 4_800],
        ),
        "generators/reactor" => (
            &[1_000_000, 4_000_000, 10_000_000, 18_000_000, 30_000_000, 50_000_000, 80_000_000],
            &[10_000, 50_000, 160_000, 100_000, 140_000, 400_000, 800_000],
            &[100, 400, 840, 1_200, 2_700, 3_800, 5_400],
        ),
        "generators/thermo_gen" => (
            &[5_000, 25_000, 100_000, 500_000, 1_000_000, 2_500_000, 8_000_000],
            &[50, 150, 300, 700, 1_000, 2_000, 4_000],
            &[3, 8, 18, 42, 90, 200, 800],
        ),
        "generators/solar_panel" => (
            &[5_000, 25_000, 100_000, 500_000, 1_000_000, 2_500_000, 8_000_000],
            &[50, 150, 300, 700, 1_000, 2_000, 4_000],
            &[5, 12, 40, 90, 220, 500, 1_200],
        ),
        _ => (&[0; 7], &[0; 7], &[]),
    };
    let mut out = Vec::new();
    for (i, t) in TIERS.iter().enumerate() {
        out.push((format!("Energy_Capacity.{t}Cap"), caps[i]));
        out.push((format!("Energy_Transfer.{t}Transfer"), transfers[i]));
        if let Some(g) = gens.get(i) {
            out.push((format!("Energy_Generation.{t}Generation"), *g));
        }
        if file == "ender_cell" || file == "ender_gate" {
            // EnderCellConfig and EnderGateConfig: Channels.
            const CHANNELS: [i64; 7] = [1, 2, 3, 5, 7, 9, 12];
            out.push((format!("Channels.{t}Channels"), CHANNELS[i]));
        }
        if file == "energy_hopper" || file == "player_transmitter" {
            // EnergyHopperConfig's Charging_Speed.
            const SPEED: [i64; 7] = [100, 1_000, 3_000, 8_000, 12_000, 20_000, 50_000];
            out.push((format!("Charging_Speed.{t}ChargingSpeed"), SPEED[i]));
        }
    }
    out
}

fn config(file: &str) -> &'static HashMap<String, i64> {
    static FILES: OnceLock<std::sync::Mutex<HashMap<String, &'static HashMap<String, i64>>>> = OnceLock::new();
    let files = FILES.get_or_init(Default::default);
    let mut files = files.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(c) = files.get(file) {
        return c;
    }
    let mut values: HashMap<String, i64> = defaults(file).into_iter().collect();
    let path = Game::native_config_dir().ok().map(|d| d.join("powah").join("energy").join(format!("{file}.toml")));
    match path.as_ref().map(std::fs::read_to_string) {
        Some(Ok(text)) => {
            let mut section = String::new();
            for line in text.lines().map(str::trim) {
                if let Some(s) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                    section = s.trim().to_owned();
                } else if let Some((k, v)) = line.split_once('=') {
                    if let Ok(n) = v.trim().parse::<i64>() {
                        values.insert(format!("{section}.{}", k.trim()), n);
                    }
                }
            }
        }
        Some(Err(_)) => {
            // As Forge writes a missing config: the defaults, by section.
            if let Some(p) = &path {
                let mut text = String::new();
                for section in ["Energy_Capacity", "Energy_Transfer", "Energy_Generation", "Charging_Speed", "Channels"] {
                    let rows: Vec<(String, i64)> = defaults(file).into_iter().filter(|(k, _)| k.starts_with(&format!("{section}."))).collect();
                    if rows.is_empty() {
                        continue;
                    }
                    text.push_str(&format!("\n[{section}]\n"));
                    for (k, v) in rows {
                        if let Some(key) = k.strip_prefix(&format!("{section}.")) {
                            text.push_str(&format!("\t{key} = {v}\n"));
                        }
                    }
                }
                let _ = p.parent().map(std::fs::create_dir_all);
                let _ = std::fs::write(p, text.trim_start());
            }
        }
        None => {}
    }
    let leaked: &'static HashMap<String, i64> = Box::leak(Box::new(values));
    files.insert(file.to_owned(), leaked);
    leaked
}

/// A tier's capacity and transfer from a config file, as ints (Horizon's buffer).
/// A generator tier's generation per tick (GeneratorConfig).
fn generation(file: &str, tier: usize) -> i32 {
    config(file).get(&format!("Energy_Generation.{}Generation", TIERS[tier])).copied().unwrap_or(0).clamp(0, i64::from(i32::MAX)) as i32
}

fn tier_values(file: &str, tier: usize) -> (i32, i32) {
    let c = config(file);
    let t = TIERS[tier];
    let get = |k: String| c.get(&k).copied().unwrap_or(0).clamp(0, i64::from(i32::MAX)) as i32;
    (get(format!("Energy_Capacity.{t}Cap")), get(format!("Energy_Transfer.{t}Transfer")))
}

/// getEnergyCapacity and getEnergyTransfer of a block.
fn energy_of(block: &str) -> Option<(i32, i32)> {
    let (kind, tier) = kind_tier(block)?;
    match (kind, tier) {
        // Energy.MAX: always full, never drained (EnergyCellTile.extractEnergy simulates for the creative cell).
        ("energy_cell", None) => Some((i32::MAX, i32::MAX)),
        ("energy_cell", Some(t)) => Some(tier_values("energy_cell", t)),
        ("energy_discharger", Some(t)) => Some(tier_values("energy_discharger", t)),
        ("energy_hopper", Some(t)) => Some(tier_values("energy_hopper", t)),
        ("energizing_rod", Some(t)) => Some(tier_values("energizing", t)),
        ("player_transmitter", Some(t)) => Some(tier_values("player_transmitter", t)),
        // The network holds the energy; the block's buffer only shows it (see ender_sync), so it is as big as an int.
        ("ender_cell", Some(t)) => Some((i32::MAX, tier_values("ender_cell", t).1)),
        ("ender_gate", Some(t)) => Some((i32::MAX, tier_values("ender_gate", t).1)),
        (_, Some(t)) if machine(block).is_some_and(Mach::generator) => Some(tier_values(machine(block)?.file(), t)),
        ("energy_cable", Some(t)) => Some(tier_values("energy_cable", t)),
        _ => None,
    }
}

// ---- NBT

fn java(s: &str) -> JavaString {
    JavaString::from(s)
}

fn get_i(nbt: &Compound, key: &str, default: i32) -> i32 {
    match nbt.get(&java(key)) {
        Some(Tag::Int(v)) => *v,
        _ => default,
    }
}

fn set_i(nbt: &mut Compound, key: &str, value: i32) {
    nbt.insert(java(key), Tag::Int(value));
}

fn energy(entity: &ModBlockEntity) -> i32 {
    get_i(&entity.nbt, "energy", 0).max(0)
}

fn set_energy(entity: &mut ModBlockEntity, value: i32) {
    let cap = energy_of(&entity.id).map_or(0, |(c, _)| c);
    set_i(&mut entity.nbt, "energy", value.clamp(0, cap));
}

// ---- Sides (Direction ids: down, up, north, south, west, east)

const FACES: [Direction; 6] = [Direction::Down, Direction::Up, Direction::North, Direction::South, Direction::West, Direction::East];
const FACE_NAMES: [&str; 6] = ["down", "up", "north", "south", "west", "east"];

fn step(pos: Pos, i: usize) -> Pos {
    let d = [[0, -1, 0], [0, 1, 0], [0, 0, -1], [0, 0, 1], [-1, 0, 0], [1, 0, 0]][i];
    [pos[0] + d[0], pos[1] + d[1], pos[2] + d[2]]
}

fn opposite(i: usize) -> usize {
    [1, 0, 3, 2, 5, 4][i]
}

/// Lollipop's Transfer: ALL, EXTRACT, RECEIVE, NONE (their ordinals are saved).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Transfer {
    All,
    Extract,
    Receive,
    None,
}

impl Transfer {
    fn from(i: i32) -> Self {
        [Self::All, Self::Extract, Self::Receive, Self::None].get(i as usize).copied().unwrap_or(Self::None)
    }
    fn extracts(self) -> bool {
        matches!(self, Self::All | Self::Extract)
    }
    fn receives(self) -> bool {
        matches!(self, Self::All | Self::Receive)
    }
    /// Transfer.next(type): the order a click steps through, by what the block allows.
    fn next(self, allowed: Transfer) -> Self {
        match allowed {
            Self::All => Self::from((self as i32 + 1) % 4),
            Self::Extract => if self != Self::None { Self::None } else { Self::Extract },
            Self::Receive => if self != Self::None { Self::None } else { Self::Receive },
            Self::None => Self::None,
        }
    }
    fn key(self) -> &'static str {
        ["all", "extract", "receive", "none"][self as usize]
    }
}

/// getTransferType: what the block's sides may be set to.
fn transfer_type(block: &str) -> Transfer {
    if is_creative(block) || machine(block).is_some_and(Mach::gives_only) { Transfer::Extract } else { Transfer::All }
}

/// isEnergyPresent: the sides that have energy at all (a solar panel's bottom only).
fn energy_present(block: &str, face: usize) -> bool {
    machine(block) != Some(Mach::Solar) || face == 0
}

/// SideConfig: `side_transfer_type`, set to the block's transfer type until first changed (init).
fn sides(entity: &ModBlockEntity) -> [Transfer; 6] {
    match entity.nbt.get(&java("side_transfer_type")) {
        Some(Tag::IntArray(a)) if a.len() == 6 => std::array::from_fn(|i| Transfer::from(a[i])),
        _ => std::array::from_fn(|i| if energy_present(&entity.id, i) { transfer_type(&entity.id) } else { Transfer::None }),
    }
}

fn set_sides(entity: &mut ModBlockEntity, s: [Transfer; 6]) {
    entity.nbt.insert(java("side_transfer_type"), Tag::IntArray(s.iter().map(|t| *t as i32).collect::<Vec<_>>().into()));
}

/// Lollipop's Redstone: IGNORE, ON, OFF (`redstone_mode`).
fn redstone_mode(entity: &ModBlockEntity) -> i32 {
    get_i(&entity.nbt, "redstone_mode", 0).clamp(0, 2)
}

/// AbstractTileEntity.checkRedstone.
fn check_redstone(game: &mut Game<'_>, entity: &ModBlockEntity, pos: Pos) -> bool {
    let mode = redstone_mode(entity);
    if mode == 0 {
        return true;
    }
    let dimension = game.native_dimension().to_owned();
    let power = game.native_neighbor_signal(&dimension, pos) > 0;
    (power && mode == 1) || (!power && mode == 2)
}

// ---- The block entity

const ORB: &str = "powah:energizing_orb";

fn is_rod(block: &str) -> bool {
    kind_tier(block).is_some_and(|(k, _)| k == "energizing_rod")
}

/// The reactor's core (`core=true`) is the machine; the other 35 blocks are parts that point at it.
fn is_reactor(block: &str) -> bool {
    kind_tier(block).is_some_and(|(k, _)| k == "reactor")
}

pub fn block_entity(block: &str, entity: &mut Option<(String, Compound)>) {
    if entity.is_none() && (machine(block).is_some() || is_cable(block) || block == ORB || is_rod(block)) {
        *entity = Some((block.to_owned(), Compound::new()));
    }
}

pub fn block_entity_ticks(block: &str, ticks: &mut bool) {
    if machine(block).is_some() || is_cable(block) || block == ORB || is_rod(block) {
        *ticks = true;
    }
}

pub fn block_entity_container(block: &str, out: &mut Option<ModContainer>) {
    if is_cable(block) {
        // A menu needs a slot: one, kept off the screen and closed to items.
        *out = Some(ModContainer { slots: 1, faces: std::array::from_fn(|_| Vec::new()), inputs: Vec::new(), outputs: Vec::new(), menu: Some("powah:energy_cable".to_owned()) });
        return;
    }
    if block == ORB {
        // EnergizingOrbTile: 7 slots, 0 the result (taken out), 1..6 the ingredients (put in, one each); no menu.
        *out = Some(ModContainer {
            slots: 7,
            faces: std::array::from_fn(|_| (0..7).collect()),
            inputs: (1..7).collect(),
            outputs: vec![0],
            menu: None,
        });
        return;
    }
    let Some(m) = machine(block) else { return };
    // The cell's two charging slots; a furnator's charging slot and its fuel; one charging slot on the others.
    let n = m.slots().len();
    let all: Vec<usize> = (0..n).collect();
    *out = Some(ModContainer {
        slots: n,
        faces: std::array::from_fn(|_| all.clone()),
        inputs: all.clone(),
        outputs: all,
        menu: Some(m.menu().to_owned()),
    });
}

/// MagmatorTile and ThermoTile: one 4,000 mB tank (`tank.setCapacity(4000)`), filled from any side.
pub fn block_entity_fluids(block: &str, out: &mut Option<horizon_client::containers::ModFluidTanks>) {
    if is_reactor(block) {
        *out = Some(horizon_client::containers::ModFluidTanks { tanks: vec![1000], faces: std::array::from_fn(|_| vec![0]), inputs: vec![0], outputs: vec![0] });
        return;
    }
    if matches!(machine(block), Some(Mach::Magmator | Mach::Thermo)) {
        *out = Some(horizon_client::containers::ModFluidTanks { tanks: vec![4000], faces: std::array::from_fn(|_| vec![0]), inputs: vec![0], outputs: vec![0] });
    }
}

/// The tank's validator: magmatic fluids in the magmator, coolants in the thermo generator.
pub fn accepts_fluid(block: &str, _tank: usize, fluid: &str, cancel: &mut bool) {
    match machine(block) {
        Some(Mach::Magmator) if magmatic_heat(fluid).is_none() => *cancel = true,
        Some(Mach::Thermo | Mach::Reactor) if coolant(fluid).is_none() => *cancel = true,
        _ => {}
    }
}

/// MagmatorConfig.magmaticFluids (heat per 100 mB): "minecraft:lava=10000" by default.
fn magmatic_heat(fluid: &str) -> Option<i32> {
    list_config("generators/magmator", "magmaticFluids", &["minecraft:lava=10000"]).get(fluid).copied()
}

/// GeneralConfig.coolantFluids (cooling per mB, -100..1): "minecraft:water=1" by default.
fn coolant(fluid: &str) -> Option<i32> {
    list_config("general_common", "coolantFluids", &["minecraft:water=1"]).get(fluid).copied()
}

/// GeneralConfig.heatSources: lava 1000, magma 800, a blazing crystal block 2800.
fn heat_source(block: &str) -> Option<i32> {
    list_config("general_common", "heatSources", &["minecraft:lava=1000", "minecraft:magma_block=800", "powah:blazing_crystal_block=2800"]).get(block).copied()
}

/// A list option of "id=number" entries from a Powah config file, or its defaults.
fn list_config(file: &str, key: &str, default: &[&str]) -> HashMap<String, i32> {
    let path = Game::native_config_dir().ok().map(|d| d.join("powah").join(if file == "general_common" { "general_common.toml".to_owned() } else { format!("energy/{file}.toml") }));
    let text = path.and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
    let entries: Vec<String> = text
        .lines()
        .find_map(|l| l.trim().strip_prefix(key).and_then(|r| r.trim().strip_prefix('=')).map(str::to_owned))
        .map(|v| v.split(['[', ']', ',', '"']).map(str::trim).filter(|s| s.contains('=')).map(str::to_owned).collect())
        .unwrap_or_else(|| default.iter().map(|s| (*s).to_owned()).collect());
    entries.iter().filter_map(|e| e.split_once('=')).filter_map(|(id, n)| Some((id.trim().to_owned(), n.trim().parse().ok()?))).collect()
}

fn tank(entity: &ModBlockEntity) -> (Option<String>, i32) {
    match entity.nbt.get(&java("fluids")) {
        Some(Tag::List(list)) => match list.iter().next() {
            Some(Tag::Compound(map)) => {
                let fluid = match map.get(&java("Fluid")) {
                    Some(Tag::String(f)) if f.to_string_lossy() != "minecraft:empty" => Some(f.to_string_lossy().to_string()),
                    _ => None,
                };
                let amount = match map.get(&java("Amount")) {
                    Some(Tag::Int(a)) => *a,
                    _ => 0,
                };
                if amount > 0 { (fluid, amount) } else { (None, 0) }
            }
            _ => (None, 0),
        },
        _ => (None, 0),
    }
}

fn set_tank(entity: &mut ModBlockEntity, fluid: Option<&str>, amount: i32) {
    let mut map = Compound::new();
    let fluid = if amount > 0 { fluid.unwrap_or("minecraft:empty") } else { "minecraft:empty" };
    map.insert(java("Fluid"), Tag::String(java(fluid)));
    map.insert(java("Amount"), Tag::Int(amount.max(0)));
    entity.nbt.insert(java("fluids"), Tag::List(vec![Tag::Compound(map)].into_iter().collect()));
}

pub fn block_entity_energy(block: &str, out: &mut Option<ModEnergyBuffer>) {
    let Some((capacity, transfer)) = energy_of(block) else { return };
    if is_cable(block) {
        // A cable holds nothing in Powah: it hands on what it gets. Here it holds one tick's transfer for other
        // mods' machines, which it hands on each tick; Powah's own machines hand to the cable's network directly.
        *out = Some(ModEnergyBuffer { capacity: transfer.max(1), max_insert: transfer, max_extract: 0, insert_faces: [true; 6], extract_faces: [false; 6] });
        return;
    }
    let m = machine(block);
    // getTransferType: a generator only gives (EXTRACT), the creative cell too; a solar panel only by its bottom
    // (isEnergyPresent: DOWN).
    let gives_only = is_creative(block) || m.is_some_and(Mach::gives_only);
    let faces: [bool; 6] = if m == Some(Mach::Solar) { [true, false, false, false, false, false] } else { [true; 6] };
    *out = Some(ModEnergyBuffer {
        capacity,
        max_insert: if gives_only { 0 } else { transfer },
        max_extract: transfer,
        insert_faces: if gives_only { [false; 6] } else { faces },
        extract_faces: faces,
    });
}

pub fn item_energy_buffer(item: &str, out: &mut Option<crate::game::native_item_energy::ItemEnergyBuffer>) {
    // EnergyCellItem keeps the cell's energy (keepEnergy); BatteryItem holds its tier's.
    let Some(name) = item.strip_prefix("powah:") else { return };
    let values = if let Some(t) = name.strip_prefix("battery_").and_then(|t| TIERS.iter().position(|x| *x == t)) {
        Some(tier_values("items/battery", t))
    } else if is_cell(item) && !is_creative(item) {
        energy_of(item)
    } else {
        None
    };
    if let Some((capacity, transfer)) = values {
        *out = Some(crate::game::native_item_energy::ItemEnergyBuffer { capacity, max_insert: transfer, max_extract: transfer });
    }
}

// ---- Cable networks (EnergyProxy.search: the cables of one tier joined to this one)

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

#[derive(Default)]
struct State {
    /// Cables whose neighbours changed: their joins are worked out again on their next tick.
    dirty: HashSet<Pos>,
    /// Each cable's network, by dimension and position, until a cable is placed or broken.
    networks: HashMap<(String, Pos), std::rc::Rc<Vec<Pos>>>,
    /// Blocks to take away on the next level tick (a hook amid a block's own change can't set blocks): dimension,
    /// place, and the block that must still be there.
    removals: Vec<(String, Pos, String)>,
    /// Reactor blocks a player broke, whose items `destroyed` already dropped.
    demolished: HashSet<Pos>,
}

/// Queues `pos` to become air on the next level tick, if `block` is still there.
fn remove_later(game: &Game<'_>, pos: Pos, block: &str) {
    let dim = game.native_dimension().to_owned();
    STATE.with_borrow_mut(|s| s.removals.push((dim, pos, block.to_owned())));
}

/// `level_tick_post`: the queued removals of this level.
pub(in crate::game) fn level_tick(game: &mut Game<'_>, dimension: &str) {
    let dim = dimension.to_owned();
    let due: Vec<(Pos, String)> = STATE.with_borrow_mut(|s| {
        let (now, later): (Vec<_>, Vec<_>) = s.removals.drain(..).partition(|(d, _, _)| *d == dim);
        s.removals = later;
        now.into_iter().map(|(_, p, b)| (p, b)).collect()
    });
    if due.is_empty() {
        return;
    }
    let Some(air) = game.native_default_state("minecraft:air") else { return };
    for (p, b) in due {
        if game.native_block(p) == b {
            game.native_set_block(&dim, p, air, 3);
        }
    }
}

/// `neighbor_changed`: a cable (or a cell) by a changed block looks at its sides again.
pub(in crate::game) fn neighbor_changed(block: &str, pos: Pos) {
    // By position alone: the level knows its dimension only as a vanilla one. Another dimension's cable at the same
    // place just looks at its sides once more.
    if is_cable(block) {
        STATE.with_borrow_mut(|s| {
            s.dirty.insert(pos);
        });
    }
}

/// The cables of `block`'s tier reached from `pos` through cables of that tier, `pos` first.
fn network(game: &Game<'_>, dimension: &str, pos: Pos, block: &str) -> std::rc::Rc<Vec<Pos>> {
    if let Some(n) = STATE.with_borrow(|s| s.networks.get(&(dimension.to_owned(), pos)).cloned()) {
        return n;
    }
    let mut seen = HashSet::from([pos]);
    let mut order = vec![pos];
    let mut i = 0;
    // A network as big as Powah's own search reaches; a bound keeps a runaway search from stalling a tick.
    while i < order.len() && order.len() < 4096 {
        let at = order[i];
        i += 1;
        for d in 0..6 {
            let next = step(at, d);
            if !seen.contains(&next) && game.native_block(next) == block {
                seen.insert(next);
                order.push(next);
            }
        }
    }
    let n = std::rc::Rc::new(order);
    STATE.with_borrow_mut(|s| {
        for p in n.iter() {
            s.networks.insert((dimension.to_owned(), *p), n.clone());
        }
    });
    n
}

fn forget_networks() {
    STATE.with_borrow_mut(|s| s.networks.clear());
}

/// CableBlock.canConnectEnergy: a neighbour that takes or gives energy through that side, and isn't a cable.
fn energy_side(game: &Game<'_>, dimension: &str, pos: Pos, d: usize) -> bool {
    let there = step(pos, d);
    if is_cable(&game.native_block(there)) {
        return false;
    }
    game.native_energy(dimension, there, FACES[opposite(d)]).is_some()
}

/// The sides of the cable at `pos` that reach energy (its `energySides`), saved as a bit mask `hz_sides`.
fn energy_sides(entity: &ModBlockEntity) -> [bool; 6] {
    let m = get_i(&entity.nbt, "hz_sides", 0);
    std::array::from_fn(|i| m & (1 << i) != 0)
}

/// CableTile.receiveEnergy: up to `amount` into the cable network at `pos` through side `from` (the side it came
/// in by, from the cable's point of view), handed straight to the machines along it. `origin` is left out (the
/// machine giving). What was taken.
fn cable_receive(game: &mut Game<'_>, cable: Pos, from: usize, amount: i32, simulate: bool) -> i32 {
    let block = game.native_block(cable);
    if !is_cable(&block) || amount <= 0 {
        return 0;
    }
    let dimension = game.native_dimension().to_owned();
    let transfer = energy_of(&block).map_or(0, |(_, t)| t);
    let net = network(game, &dimension, cable, &block);
    let mut received = 0;
    for &p in net.iter() {
        let Some(sides) = cable_state(game, p) else { continue };
        if !sides.1 {
            continue; // checkRedstone of that cable
        }
        for d in 0..6 {
            let left = (amount - received).min(transfer);
            if left <= 0 {
                return received;
            }
            // pushEnergy: not back out where it came in, and only by sides set to give.
            if (p == cable && d == from) || !sides.0[d] {
                continue;
            }
            let there = step(p, d);
            if refuses(game, there, opposite(d)) {
                continue;
            }
            received += game.native_energy_insert(there, FACES[opposite(d)], left, simulate);
        }
    }
    received
}

/// Whether a Powah block at `pos` refuses energy coming in by its side `face` (canReceiveEnergy: its side config).
/// Other mods' blocks answer through their buffer.
fn refuses(game: &Game<'_>, pos: Pos, face: usize) -> bool {
    let Some(horizon_client::containers::BlockEntity::Mod(e)) = game.world.block_entities.get(&(pos[0], pos[1], pos[2])) else { return false };
    machine(&e.id).is_some() && !sides(e)[face].receives()
}

/// A cable's (sides it gives by, redstone lets it work), read from its block entity.
fn cable_state(game: &mut Game<'_>, pos: Pos) -> Option<([bool; 6], bool)> {
    let horizon_client::containers::BlockEntity::Mod(e) = game.world.block_entities.get(&(pos[0], pos[1], pos[2]))? else { return None };
    let e = (**e).clone();
    let s = sides(&e);
    let joined = energy_sides(&e);
    let gives: [bool; 6] = std::array::from_fn(|d| joined[d] && s[d].extracts());
    let on = check_redstone(game, &e, pos);
    Some((gives, on))
}

/// Gives energy out of `entity` (at `pos`) by its sides set to give: into a cable's network or a machine.
/// AbstractEnergyStorage.extractFromSides.
fn extract_from_sides(game: &mut Game<'_>, entity: &mut ModBlockEntity, pos: Pos, transfer: i32) -> i32 {
    let s = sides(entity);
    let creative = is_creative(&entity.id);
    let mut total = 0;
    for d in 0..6 {
        if !s[d].extracts() {
            continue;
        }
        let have = if creative { transfer } else { energy(entity) };
        let amount = transfer.min(have);
        if amount <= 0 {
            break;
        }
        let there = step(pos, d);
        let moved = if refuses(game, there, opposite(d)) {
            0
        } else if is_cable(&game.native_block(there)) {
            cable_receive(game, there, opposite(d), amount, false)
        } else {
            game.native_energy_insert(there, FACES[opposite(d)], amount, false)
        };
        if !creative {
            { let left = energy(entity) - moved; set_energy(entity, left); }
        }
        total += moved;
    }
    total
}

/// AbstractEnergyStorage.chargeItems: the cell's slots, each up to the transfer.
fn charge_items(game: &mut Game<'_>, entity: &mut ModBlockEntity, transfer: i32) -> i32 {
    let n = entity.items.len();
    charge_items_from(game, entity, 0, n, transfer)
}

/// chargeItems(i, j): slots i..j.
fn charge_items_from(game: &mut Game<'_>, entity: &mut ModBlockEntity, from: usize, to: usize, transfer: i32) -> i32 {
    let creative = is_creative(&entity.id);
    let mut total = 0;
    for slot in from..to.min(entity.items.len()) {
        let Some(stack) = entity.items[slot].clone() else { continue };
        let amount = if creative { transfer } else { transfer.min(energy(entity)) };
        if amount <= 0 {
            break;
        }
        if let Some((moved, charged)) = game.native_item_energy_insert(&stack, amount) {
            if moved > 0 {
                entity.items[slot] = Some(charged);
                if !creative {
                    { let left = energy(entity) - moved; set_energy(entity, left); }
                }
                total += moved;
            }
        }
    }
    total
}

// Menu data slots: what the screen reads (16 bits each, so ints go in two).
const D_ENERGY: usize = 0; // and 1
const D_CAPACITY: usize = 2; // and 3
const D_TRANSFER: usize = 4; // and 5
const D_SIDES: usize = 6; // 6 sides
const D_REDSTONE: usize = 12;
const D_CREATIVE: usize = 13;
const D_GENERATION: usize = 14; // and 15: getGeneration, or what a thermo generator makes now
const D_FUEL: usize = 16; // and 17: the furnator's carbon, the magmator's buffer
const D_FUEL_MAX: usize = 18; // and 19
const D_BURNING: usize = 20;
const D_FLUID: usize = 21; // 0 none, 1 water, 2 lava, 3 another
const D_FLUID_AMOUNT: usize = 22;
const D_SKY: usize = 23; // the solar panel sees the sky (or has its lens)
const D_CHANNEL: usize = 24;
const D_CHANNELS: usize = 25;
const D_R_FUEL: usize = 26; // fuel, carbon, redstone, solid coolant, temperature: ticks and max, in hundredths, wide
const D_R_GEN_MODE: usize = 46;
const D_R_PART: usize = 47;
const DATA_SLOTS: usize = 48;

fn put_wide(data: &mut [i32], at: usize, v: i32) {
    data[at] = v & 0xFFFF;
    data[at + 1] = (v >> 16) & 0xFFFF;
}

fn wide(data: &[i32], at: usize) -> i32 {
    (data[at] & 0xFFFF) | ((data[at + 1] & 0xFFFF) << 16)
}

fn generation_of(block: &str) -> i32 {
    match (machine(block), kind_tier(block)) {
        (Some(m), Some((_, Some(t)))) if m.generator() => generation(m.file(), t),
        _ => 0,
    }
}

fn fill_cable_data(entity: &mut ModBlockEntity) {
    if entity.data.len() < DATA_SLOTS {
        entity.data.resize(DATA_SLOTS, 0);
    }
    let s = sides(entity);
    for (i, t) in s.iter().enumerate() {
        entity.data[D_SIDES + i] = *t as i32;
    }
    entity.data[D_REDSTONE] = redstone_mode(entity);
    entity.data[D_CHANNEL] = get_i(&entity.nbt, "hz_menu_side", 2).clamp(0, 5);
}

fn fill_data(entity: &mut ModBlockEntity) {
    let (cap, transfer) = energy_of(&entity.id).unwrap_or((0, 0));
    let creative = is_creative(&entity.id);
    if entity.data.len() < DATA_SLOTS {
        entity.data.resize(DATA_SLOTS, 0);
    }
    let stored = if creative { cap } else { energy(entity) };
    let s = sides(entity);
    let mode = redstone_mode(entity);
    if entity.data.len() < DATA_SLOTS {
        entity.data.resize(DATA_SLOTS, 0);
    }
    put_wide(&mut entity.data, D_ENERGY, stored);
    put_wide(&mut entity.data, D_CAPACITY, cap);
    put_wide(&mut entity.data, D_TRANSFER, transfer);
    for (i, t) in s.iter().enumerate() {
        entity.data[D_SIDES + i] = *t as i32;
    }
    entity.data[D_REDSTONE] = mode;
    entity.data[D_CREATIVE] = i32::from(creative);
    let generating = if machine(&entity.id) == Some(Mach::Thermo) { get_i(&entity.nbt, "generating", 0) } else { generation_of(&entity.id) };
    put_wide(&mut entity.data, D_GENERATION, generating);
    { let v = get_i(&entity.nbt, "hz_fuel", 0); put_wide(&mut entity.data, D_FUEL, v); }
    { let v = get_i(&entity.nbt, "hz_fuel_max", 0); put_wide(&mut entity.data, D_FUEL_MAX, v); }
    entity.data[D_BURNING] = if machine(&entity.id) == Some(Mach::Transmitter) {
        i32::from(entity.items.first().cloned().flatten().is_some_and(|c| bound(&c).is_some()))
    } else {
        get_i(&entity.nbt, "burning", 0)
    };
    let (fluid, amount) = tank(entity);
    entity.data[D_FLUID] = match fluid.as_deref() {
        None => 0,
        Some("minecraft:water") => 1,
        Some("minecraft:lava") => 2,
        Some(_) => 3,
    };
    entity.data[D_FLUID_AMOUNT] = amount;
    entity.data[D_SKY] = get_i(&entity.nbt, "can_see_sky", 0) | get_i(&entity.nbt, "has_lens_of_ender", 0);
    if machine(&entity.id) == Some(Mach::Reactor) {
        for (i, key) in ["fuel", "carbon", "redstone", "solid_coolant", "temperature"].iter().enumerate() {
            let v = get_i(&entity.nbt, key, 0);
            let m = get_i(&entity.nbt, &format!("{key}_max"), 0);
            put_wide(&mut entity.data, D_R_FUEL + i * 4, v);
            put_wide(&mut entity.data, D_R_FUEL + i * 4 + 2, m);
        }
        entity.data[D_R_GEN_MODE] = get_i(&entity.nbt, "gen_mode", 0);
        entity.data[D_R_PART] = get_i(&entity.nbt, "hz_part", 0);
    }
    if matches!(machine(&entity.id), Some(Mach::EnderCell | Mach::EnderGate)) {
        entity.data[D_CHANNEL] = channel(entity);
        entity.data[D_CHANNELS] = max_channels(entity);
        let cap = get_i(&entity.nbt, "hz_net_cap", 0);
        put_wide(&mut entity.data, D_CAPACITY, cap);
    }
}

pub(in crate::game) fn block_entity_tick(game: &mut Game<'_>, pos: Pos, entity: &mut ModBlockEntity) {
    if !game.native_on(NS) {
        return;
    }
    block_entity_tick_inner(game, pos, entity);
    sync_visuals(game, entity, pos);
}

fn block_entity_tick_inner(game: &mut Game<'_>, pos: Pos, entity: &mut ModBlockEntity) {
    let block = entity.id.clone();
    if block == ORB {
        orb_tick(game, entity);
        return;
    }
    if is_rod(&block) {
        rod_tick(game, entity, pos);
        return;
    }
    let Some((cap, transfer)) = energy_of(&block) else { return };
    let dimension = game.native_dimension().to_owned();
    if is_cable(&block) {
        // CableBlock: the sides that reach energy, worked out when placed and when a neighbour changes.
        let dirty = STATE.with_borrow_mut(|s| s.dirty.remove(&pos));
        if dirty || !entity.nbt.contains_key(&java("hz_sides")) {
            let mut mask = 0;
            for d in 0..6 {
                let there = step(pos, d);
                let cable = game.native_block(there) == block;
                let machine = energy_side(game, &dimension, pos, d);
                if machine {
                    mask |= 1 << d;
                }
                let joined = if cable || machine { "true" } else { "false" };
                if game.native_block_property(pos, FACE_NAMES[d]).as_deref().is_some_and(|v| v != joined) {
                    game.native_set_block_property_with(pos, FACE_NAMES[d], joined, 3);
                }
            }
            set_i(&mut entity.nbt, "hz_sides", mask);
            forget_networks();
        }
        fill_cable_data(entity);
        // What other mods put into the cable's buffer goes on to the network this tick.
        let held = energy(entity);
        if held > 0 && check_redstone(game, entity, pos) {
            let moved = cable_receive(game, pos, 6, held, false);
            set_energy(entity, held - moved);
        }
        return;
    }
    // EnergyCellTile.postTick: charge the slots, then give by the sides; the creative cell stays full.
    if is_creative(&block) {
        set_i(&mut entity.nbt, "energy", cap);
    }
    let powered = check_redstone(game, entity, pos);
    match machine(&block) {
        Some(Mach::Cell) => {
            if powered {
                charge_items(game, entity, transfer);
                extract_from_sides(game, entity, pos, transfer);
            }
        }
        Some(Mach::Hopper) => {
            if powered {
                hopper_charge(game, entity, pos);
            }
        }
        Some(Mach::Transmitter) => {
            if powered {
                transmit(game, entity);
            }
        }
        Some(m @ (Mach::EnderCell | Mach::EnderGate)) => {
            ender_sync(game, entity);
            if m == Mach::EnderCell {
                ender_extend(game, entity);
            }
            if powered {
                charge_items_from(game, entity, 1, 3, transfer);
                extract_from_sides(game, entity, pos, transfer);
            }
            ender_sync(game, entity);
        }
        Some(Mach::Discharger) => {
            // EnergyDischargerTile.postTick: from each slot's item, up to the transfer and the room left.
            if powered {
                for slot in 0..entity.items.len() {
                    let Some(stack) = entity.items[slot].clone() else { continue };
                    let room = cap - energy(entity);
                    let want = transfer.min(room);
                    if want <= 0 {
                        break;
                    }
                    if let Some((took, drained)) = game.native_item_energy_extract(&stack, want) {
                        if took > 0 {
                            entity.items[slot] = Some(drained);
                            let now = energy(entity) + took;
                            set_energy(entity, now);
                        }
                    }
                }
            }
            extract_from_sides(game, entity, pos, transfer);
        }
        Some(Mach::Reactor) => {
            if get_i(&entity.nbt, "hz_part", 0) != 0 {
                return;
            }
            if reactor_build(game, entity, pos) {
                reactor_parts_forward(game, entity, pos);
                reactor_tick(game, entity, pos, cap, transfer, powered);
                reactor_parts_mirror(game, entity, pos);
            }
        }
        Some(m) => {
            if powered {
                generate(game, entity, pos, m, cap);
            }
            // The generators charge and give whatever the redstone says (postTick's last line).
            charge_items_from(game, entity, 0, 1, transfer);
            extract_from_sides(game, entity, pos, transfer);
        }
        None => {}
    }
    fill_data(entity);
}

// ---- Generators (FurnatorTile, MagmatorTile, ThermoTile, SolarTile)

/// GeneralConfig.fuelTicks: energy per solid fuel tick (30: coal's 1,600 ticks make 48,000 FE).
fn fuel_ticks() -> i64 {
    // powah/general_common.toml, [Materials] solidFuelEnergyBase = 30.
    static TICKS: OnceLock<i64> = OnceLock::new();
    *TICKS.get_or_init(|| {
        let text = Game::native_config_dir().ok().and_then(|d| std::fs::read_to_string(d.join("powah").join("general_common.toml")).ok()).unwrap_or_default();
        text.lines()
            .filter_map(|l| l.split_once('='))
            .find(|(k, _)| k.trim() == "solidFuelEnergyBase")
            .and_then(|(_, v)| v.trim().parse().ok())
            .unwrap_or(30i64)
            .clamp(1, 1_000_000)
    })
}

fn generate(game: &mut Game<'_>, entity: &mut ModBlockEntity, pos: Pos, m: Mach, cap: i32) {
    let per_tick = generation_of(&entity.id);
    let stored = energy(entity);
    let mut burning = false;
    match m {
        Mach::Furnator => {
            // The carbon (in FE): a fuel's burn time by fuelTicks, taken one item at a time.
            let mut carbon = i64::from(get_i(&entity.nbt, "hz_fuel", 0));
            if carbon <= 0 {
                if let Some(stack) = entity.items.get(1).cloned().flatten() {
                    let burn = game.res.providers().burn_time(game.res.items(), &stack, false) as i64;
                    if burn > 0 {
                        carbon = (burn * fuel_ticks()).min(i64::from(i32::MAX));
                        set_i(&mut entity.nbt, "hz_fuel_max", carbon as i32);
                        let rest = game.res.items().get(&stack.item).and_then(|i| i.crafting_remainder.as_deref()).map(|r| ItemStack::new(r, 1));
                        entity.items[1] = match rest {
                            Some(r) => Some(r),
                            None if stack.count > 1 => Some(ItemStack { count: stack.count - 1, ..stack }),
                            None => None,
                        };
                    }
                }
            }
            if carbon > 0 && stored < cap {
                let make = (cap - stored).min(per_tick).min(carbon as i32);
                set_energy(entity, stored + make);
                carbon -= i64::from(make);
                burning = true;
            }
            set_i(&mut entity.nbt, "hz_fuel", carbon.max(0) as i32);
        }
        Mach::Magmator => {
            // The buffer: up to 100 mB at a time, heat per 100 mB.
            let mut buffer = get_i(&entity.nbt, "hz_fuel", 0);
            if buffer <= 0 {
                let (fluid, amount) = tank(entity);
                if let Some(heat) = fluid.as_deref().and_then(magmatic_heat) {
                    let take = amount.min(100);
                    buffer = ((i64::from(take) * i64::from(heat)) / 100) as i32;
                    set_i(&mut entity.nbt, "hz_fuel_max", buffer);
                    set_tank(entity, fluid.as_deref(), amount - take);
                }
            }
            let make = per_tick.min(buffer);
            if make > 0 && cap - stored >= make {
                set_energy(entity, stored + make);
                buffer -= make;
                burning = true;
            }
            set_i(&mut entity.nbt, "hz_fuel", buffer.max(0));
        }
        Mach::Thermo => {
            let mut generating = 0;
            let (fluid, amount) = tank(entity);
            if let Some(cooling) = fluid.as_deref().and_then(coolant) {
                let below = [pos[0], pos[1] - 1, pos[2]];
                let block = game.native_block(below);
                if stored < cap {
                    if let Some(mut heat) = heat_source(&block) {
                        // A flowing fluid's heat falls off with its level.
                        if block == "minecraft:lava" || block == "minecraft:water" {
                            let level: i32 = game.native_block_property(below, "level").and_then(|l| l.parse().ok()).unwrap_or(0);
                            if level != 0 {
                                heat = (heat as f32 / (level as f32 + 1.0)) as i32;
                            }
                        }
                        let factor = if cooling == 1 { 1.0 } else { (1.1f64).max((0.1 + f64::from(cooling.abs())) * 1.1152) };
                        generating = (f64::from(heat) * factor * f64::from(per_tick) / 1000.0) as i32;
                        set_energy(entity, stored + generating);
                        if { let n = get_i(&entity.nbt, "hz_ticks", 0).wrapping_add(1); set_i(&mut entity.nbt, "hz_ticks", n); n % 40 == 0 } {
                            set_tank(entity, fluid.as_deref(), amount - 1);
                        }
                    }
                }
            }
            set_i(&mut entity.nbt, "generating", generating);
        }
        Mach::Solar => {
            let lens = get_i(&entity.nbt, "has_lens_of_ender", 0) != 0;
            let ticks = get_i(&entity.nbt, "hz_ticks", 0).wrapping_add(1);
            set_i(&mut entity.nbt, "hz_ticks", ticks);
            if !lens && (ticks % 40 == 1) {
                // Misc.canBlockSeeSky over the panel.
                let sky = game.native_light([pos[0], pos[1] + 1, pos[2]]).1 >= 15;
                set_i(&mut entity.nbt, "can_see_sky", i32::from(sky));
            }
            let sees = lens || get_i(&entity.nbt, "can_see_sky", 0) != 0;
            // World.isDaytime: the overworld's day (the sky not darkened past 4 of 15).
            let t = game.native_day_time().rem_euclid(24_000);
            let day = game.native_dimension() == "minecraft:overworld" && !(12_542..23_460).contains(&t);
            if stored < cap && sees && day {
                set_energy(entity, stored + per_tick);
                burning = true;
            }
        }
        Mach::Cell | Mach::Discharger | Mach::Hopper | Mach::Transmitter | Mach::EnderCell | Mach::EnderGate | Mach::Reactor => {}
    }
    set_i(&mut entity.nbt, "burning", i32::from(burning));
}

// ---- The energy hopper (EnergyHopperTile.postTick)

/// Charges each single item in the inventory the hopper faces, up to its charging speed, where it lies.
fn hopper_charge(game: &mut Game<'_>, entity: &mut ModBlockEntity, pos: Pos) {
    let Some((_, Some(tier))) = kind_tier(&entity.id) else { return };
    let speed = config("energy_hopper").get(&format!("Charging_Speed.{}ChargingSpeed", TIERS[tier])).copied().unwrap_or(0).clamp(0, i64::from(i32::MAX)) as i32;
    let Some(facing) = game.native_block_property(pos, "facing").and_then(|f| FACE_NAMES.iter().position(|n| *n == f)) else { return };
    let there = step(pos, facing);
    let dimension = game.native_dimension().to_owned();
    let Some(slots) = game.native_container_slots(&dimension, there, FACES[opposite(facing)]) else { return };
    for s in slots {
        let Some(stack) = s.stack else { continue };
        // `stack.getCount() == 1`: a stack of more isn't charged.
        if stack.count != 1 {
            continue;
        }
        let amount = speed.min(energy(entity));
        if amount <= 0 {
            break;
        }
        if let Some((moved, charged)) = game.native_item_energy_insert(&stack, amount) {
            if moved > 0 && game.powah_replace_slot(&dimension, there, s.slot, &stack, charged) {
                let left = energy(entity) - moved;
                set_energy(entity, left);
            }
        }
    }
}

// ---- Binding cards and the player transmitter (BindingCardItem, PlayerTransmitterTile)

const CARD: &str = "powah:binding_card";
const CARD_DIM: &str = "powah:binding_card_dim";

fn custom(stack: &ItemStack) -> Option<&Compound> {
    match stack.extra.as_ref()?.other.iter().find(|(k, _)| k == "minecraft:custom_data").map(|(_, t)| t) {
        Some(Tag::Compound(c)) => Some(c),
        _ => None,
    }
}

/// The card's `bound_player_id` (a UUID, saved as Java saves one: four ints) and `bound_player_name`.
fn bound(stack: &ItemStack) -> Option<(u128, String)> {
    let c = custom(stack)?;
    let Some(Tag::IntArray(a)) = c.get(&java("bound_player_id")) else { return None };
    if a.len() != 4 {
        return None;
    }
    let id = a.iter().fold(0u128, |acc, v| (acc << 32) | u128::from(*v as u32));
    let name = match c.get(&java("bound_player_name")) {
        Some(Tag::String(s)) => s.to_string_lossy().to_string(),
        _ => String::new(),
    };
    Some((id, name))
}

/// `use_item`: a card used in the air binds to its user, once (BindingCardItem.onItemRightClick); on the server.
pub(in crate::game) fn use_item(game: &mut Game<'_>, stack: &ItemStack, claimed: &mut bool) {
    if &*stack.item == WRENCH {
        // WrenchItem.onItemRightClick: sneaking, the next mode, said on the action bar.
        let Some(player) = game.native_acting_player() else { return };
        if !game.native_sneaking() {
            return;
        }
        let slot = game.selected as i32;
        let mut next = 0;
        if let Some(h) = game.native_carried_stack(slot).filter(|h| &*h.item == WRENCH) {
            next = (wrench_mode(h) + 1) % 3;
            set_wrench_tag(h, |c| {
                c.insert(java("WrenchMode"), Tag::Int(next as i32));
            });
        }
        game.native_action_bar(player.uuid, horizon_client::text::Text::legacy(&format!("§7{}", wrench_line(next))));
        *claimed = true;
        return;
    }
    if is_battery(&stack.item) {
        // BatteryItem.onItemRightClick: sneaking switches charging (shown as the enchantment glint).
        // isSneaking: the acting player's sneak (a LAN guest's too).
        let sneaking = game.native_sneaking();
        if !sneaking || game.native_acting_player().is_none() {
            return;
        }
        let slot = game.selected as i32;
        if let Some(held) = game.native_carried_stack(slot).filter(|h| is_battery(&h.item)) {
            let on = !charging(held);
            let mut c = custom(held).cloned().unwrap_or_default();
            c.insert(java("charging"), Tag::Byte(i8::from(on)));
            let extra = held.extra.get_or_insert_with(Default::default);
            extra.other.retain(|(k, _)| k != "minecraft:custom_data" && k != "minecraft:enchantment_glint_override");
            extra.other.push(("minecraft:custom_data".into(), Tag::Compound(c)));
            if on {
                extra.other.push(("minecraft:enchantment_glint_override".into(), Tag::Byte(1)));
            }
            *claimed = true;
        }
        return;
    }
    if &*stack.item != CARD && &*stack.item != CARD_DIM {
        return;
    }
    let Some(player) = game.native_acting_player() else { return };
    match bound(stack) {
        None => {
            let slot = game.selected as i32;
            let Some(held) = game.native_carried_stack(slot) else { return };
            if &*held.item != &*stack.item {
                return;
            }
            let mut c = custom(held).cloned().unwrap_or_default();
            let ints: Vec<i32> = (0..4).map(|i| (player.uuid >> (96 - 32 * i)) as u32 as i32).collect();
            c.insert(java("bound_player_id"), Tag::IntArray(ints.into()));
            c.insert(java("bound_player_name"), Tag::String(java(&player.name)));
            let extra = held.extra.get_or_insert_with(Default::default);
            extra.other.retain(|(k, _)| k != "minecraft:custom_data");
            extra.other.push(("minecraft:custom_data".into(), Tag::Compound(c)));
            *claimed = true;
        }
        Some((id, name)) if id != player.uuid => {
            // "chat.powah.no.binding": someone else's card.
            let text = lang("chat.powah.no.binding", "This card is bound to %s").replacen("%s", &name, 1);
            game.native_action_bar(player.uuid, horizon_client::text::Text::legacy(&format!("§4{text}")));
            *claimed = true;
        }
        Some(_) => {}
    }
}

/// An energy item's stored energy and its buffer (capacity, transfer), from the stack alone (the client's view).
fn item_energy(stack: &ItemStack) -> Option<(i64, i32, i32)> {
    let mut buffer = None;
    item_energy_buffer(&stack.item, &mut buffer);
    let b = buffer?;
    let stored = match custom(stack).and_then(|c| c.get(&java("horizon_energy"))) {
        Some(Tag::Int(v)) => i64::from(*v),
        _ => 0,
    };
    Some((stored, b.capacity, b.max_extract))
}

fn is_battery(item: &str) -> bool {
    item.strip_prefix("powah:").is_some_and(|n| n.starts_with("battery_"))
}

fn charging(stack: &ItemStack) -> bool {
    matches!(custom(stack).and_then(|c| c.get(&java("charging"))), Some(Tag::Byte(1)))
}

/// `item_bar`: BatteryItem.showDurabilityBar (while not full) and Forge's colour (green to red as it empties).
pub fn item_bar(stack: &ItemStack, bar: &mut Option<(i32, u32)>) {
    if !is_battery(&stack.item) {
        return;
    }
    let Some((stored, cap, _)) = item_energy(stack) else { return };
    if cap <= 0 || stored >= i64::from(cap) {
        return;
    }
    let durability = 1.0 - stored as f64 / f64::from(cap);
    let width = (13.0 - durability * 13.0).round().clamp(0.0, 13.0) as i32;
    // MathHelper.hsvToRGB(max(0, 1 - durability) / 3, 1, 1).
    let hue = (1.0 - durability).max(0.0) / 3.0;
    let h6 = hue * 6.0;
    let x = 1.0 - (h6 % 2.0 - 1.0).abs();
    let (r, g, b) = if h6 < 1.0 { (1.0, x, 0.0) } else { (x, 1.0, 0.0) };
    let rgb = ((r * 255.0) as u32) << 16 | ((g * 255.0) as u32) << 8 | (b * 255.0) as u32;
    *bar = Some((width, rgb));
}

/// The card's tooltip: who owns it, or how to bind it; an energy item's stored energy and its I/O (EnergyItem).
pub(in crate::game) fn tooltip(stack: &ItemStack, lines: &mut Vec<(horizon_client::text::Text, u32)>) {
    if &*stack.item == WRENCH {
        lines.push((horizon_client::text::Text::legacy(&format!("§7{}", wrench_line(wrench_mode(stack)))), u32::MAX));
        return;
    }
    if is_battery(&stack.item) {
        if let Some((stored, cap, io)) = item_energy(stack).filter(|e| e.1 > 0) {
            let s = lang("info.lollipop.fe.stored", "%s / %s FE").replacen("%s", &add_commas(stored), 1).replacen("%s", &num_format(i64::from(cap)), 1);
            lines.push((horizon_client::text::Text::legacy(&format!("§7{}: §8{s}", lang("info.lollipop.stored", "Stored"))), u32::MAX));
            let m = lang("info.lollipop.fe.pet.tick", "%s FE/t").replacen("%s", &num_format(i64::from(io)), 1);
            lines.push((horizon_client::text::Text::legacy(&format!("§7{}: §8{m}", lang("info.lollipop.max.io", "Max I/O"))), u32::MAX));
            lines.push((horizon_client::text::Text::literal(""), u32::MAX));
        }
        return;
    }
    if &*stack.item != CARD && &*stack.item != CARD_DIM {
        return;
    }
    let line = match bound(stack) {
        Some((_, name)) => format!("§7{}", lang("info.lollipop.owner", "Owner: %s").replacen("%s", &format!("§e{name}"), 1)),
        None => format!("§8{}", lang("info.powah.click.to.bind", "Right click to bind")),
    };
    lines.push((horizon_client::text::Text::legacy(&line), u32::MAX));
    lines.push((horizon_client::text::Text::literal(""), u32::MAX));
}

/// PlayerTransmitterTile.postTick: the bound player's items, each up to the charging speed, if they're in this
/// dimension (any, with a dimensional card).
fn transmit(game: &mut Game<'_>, entity: &mut ModBlockEntity) {
    let Some(card) = entity.items.first().cloned().flatten() else { return };
    let Some((id, _)) = bound(&card) else { return };
    let Some((_, Some(tier))) = kind_tier(&entity.id) else { return };
    let here = game.native_dimension().to_owned();
    let Some(player) = game.native_server_players().into_iter().find(|p| p.uuid == id) else { return };
    if &*card.item != CARD_DIM && player.dimension != here {
        return;
    }
    let speed = config("player_transmitter").get(&format!("Charging_Speed.{}ChargingSpeed", TIERS[tier])).copied().unwrap_or(0).clamp(0, i64::from(i32::MAX)) as i32;
    let kind = crate::game::native_inventories::NativeInventoryKind::Player;
    let Ok(inv) = game.native_inventory(id, kind) else { return };
    for (slot, stack) in inv.slots.into_iter().enumerate() {
        let Some(stack) = stack else { continue };
        let amount = speed.min(energy(entity));
        if amount <= 0 {
            break;
        }
        if let Some((moved, charged)) = game.native_item_energy_insert(&stack, amount) {
            if moved > 0 && game.native_set_inventory_slot(id, kind, slot, Some(&stack), Some(charged)).is_ok() {
                let left = energy(entity) - moved;
                set_energy(entity, left);
            }
        }
    }
}

// ---- Energizing (EnergizingOrbTile, EnergizingRodTile, EnergizingRecipe)

/// EnergizingConfig.range: how far a rod reaches an orb (4).
fn energizing_range() -> i32 {
    static RANGE: OnceLock<i32> = OnceLock::new();
    *RANGE.get_or_init(|| {
        let text = Game::native_config_dir().ok().and_then(|d| std::fs::read_to_string(d.join("powah").join("energy").join("energizing.toml")).ok()).unwrap_or_default();
        text.lines().filter_map(|l| l.split_once('=')).find(|(k, _)| k.trim() == "range").and_then(|(_, v)| v.trim().parse().ok()).unwrap_or(4).clamp(1, 32)
    })
}

/// The orb's ingredients as a key, to look for a recipe only when they change.
fn orb_key(entity: &ModBlockEntity) -> String {
    entity.items.iter().skip(1).map(|s| s.as_ref().map_or(String::new(), |s| format!("{:?}", s))).collect::<Vec<_>>().join("|")
}

/// EnergizingRecipe.matches: each ingredient in slots 1..6 takes one of the recipe's, and none are left over.
fn orb_recipe(game: &Game<'_>, entity: &ModBlockEntity) -> Option<(String, i64, ItemStack)> {
    let stacks: Vec<ItemStack> = entity.items.iter().skip(1).filter_map(|s| s.clone()).collect();
    if stacks.is_empty() {
        return None;
    }
    for r in game.native_recipes_of_type("powah:energizing").iter() {
        let Some(list) = r.json["ingredients"].as_array() else { continue };
        let mut left: Vec<Vec<String>> = list
            .iter()
            .map(|i| {
                let j = match i {
                    serde_json::Value::String(s) => match s.strip_prefix('#') {
                        Some(tag) => serde_json::json!({ "tag": tag }),
                        None => serde_json::json!({ "item": s }),
                    },
                    other => other.clone(),
                };
                game.native_recipe_ingredient(&j).map(|ing| ing.order.iter().map(|s| s.to_string()).collect()).unwrap_or_else(|| j["item"].as_str().map(|s| vec![s.to_owned()]).unwrap_or_default())
            })
            .collect();
        let all = stacks.iter().all(|s| match left.iter().position(|ids| ids.iter().any(|id| *id == *s.item)) {
            Some(i) => {
                left.remove(i);
                true
            }
            None => false,
        });
        if all && left.is_empty() {
            let result = &r.json["result"];
            let id = result["id"].as_str().or_else(|| result["item"].as_str()).unwrap_or("minecraft:air");
            let count = result["count"].as_u64().unwrap_or(1).clamp(1, 64) as u32;
            let energy = r.json["energy"].as_i64().unwrap_or(0).max(1);
            return Some((r.id.to_string(), energy, ItemStack::new(id, count)));
        }
    }
    None
}

/// The orb (onSlotChanged, checkRecipe): its buffer is the recipe's energy, emptied when what it holds changes.
fn orb_tick(game: &mut Game<'_>, entity: &mut ModBlockEntity) {
    if entity.items.len() < 7 {
        entity.items.resize(7, None);
    }
    let key = orb_key(entity);
    let old = match entity.nbt.get(&java("hz_key")) {
        Some(Tag::String(s)) => s.to_string_lossy().to_string(),
        _ => String::from("?"),
    };
    if key != old {
        entity.nbt.insert(java("hz_key"), Tag::String(java(&key)));
        set_i(&mut entity.nbt, "hz_buffer", 0);
        let cap = orb_recipe(game, entity).map_or(0, |(_, e, _)| e.min(i64::from(i32::MAX)) as i32);
        set_i(&mut entity.nbt, "hz_buffer_cap", cap);
        // containRecipe: the rods' beams and the client's floating items read it.
        set_i(&mut entity.nbt, "contain_recipe", i32::from(cap > 0));
    }
}

/// EnergizingOrbTile.fillEnergy: up to `amount` into the orb at `orb`; when full, the result replaces the ingredients.
fn orb_fill(game: &mut Game<'_>, orb: Pos, amount: i32) -> i32 {
    let recipe = {
        let Some(horizon_client::containers::BlockEntity::Mod(e)) = game.world.block_entities.get(&(orb[0], orb[1], orb[2])) else { return 0 };
        if get_i(&e.nbt, "hz_buffer_cap", 0) <= 0 {
            return 0;
        }
        orb_recipe(game, e)
    };
    let Some((_, _, result)) = recipe else { return 0 };
    game.native_with_mod_block_entity(orb, |e| {
        let cap = get_i(&e.nbt, "hz_buffer_cap", 0);
        let have = get_i(&e.nbt, "hz_buffer", 0);
        let filled = amount.min(cap - have).max(0);
        let now = have + filled;
        if now >= cap {
            e.items = vec![None; 7];
            e.items[0] = Some(result.clone());
            set_i(&mut e.nbt, "hz_buffer", 0);
            set_i(&mut e.nbt, "hz_buffer_cap", 0);
            set_i(&mut e.nbt, "contain_recipe", 0);
            let key = orb_key(e);
            e.nbt.insert(java("hz_key"), Tag::String(java(&key)));
        } else {
            set_i(&mut e.nbt, "hz_buffer", now);
        }
        filled
    })
    .unwrap_or(0)
}

/// The rod (EnergizingRodTile.postTick): with an orb that holds a recipe, it warms up for 20 ticks, then pours its
/// energy in, up to its transfer a tick. Its orb is the first one in range, found again when it's gone.
fn rod_tick(game: &mut Game<'_>, entity: &mut ModBlockEntity, pos: Pos) {
    let Some((_, transfer)) = energy_of(&entity.id) else { return };
    let mut orb = match entity.nbt.get(&java("OrbPos")) {
        Some(Tag::IntArray(a)) if a.len() == 3 => Some([a[0], a[1], a[2]]),
        _ => None,
    };
    if orb.is_some_and(|o| game.native_block(o) != ORB) {
        orb = None;
    }
    if orb.is_none() {
        // EnergizingRodBlock.setOrbPos: the first orb in the cube around it.
        let r = energizing_range();
        'search: for x in -r..=r {
            for y in -r..=r {
                for z in -r..=r {
                    let p = [pos[0] + x, pos[1] + y, pos[2] + z];
                    if game.native_block(p) == ORB {
                        orb = Some(p);
                        break 'search;
                    }
                }
            }
        }
        match orb {
            Some(o) => entity.nbt.insert(java("OrbPos"), Tag::IntArray(vec![o[0], o[1], o[2]].into())),
            None => entity.nbt.remove(&java("OrbPos")),
        };
    }
    let Some(orb) = orb else { return };
    let contains = match game.world.block_entities.get(&(orb[0], orb[1], orb[2])) {
        Some(horizon_client::containers::BlockEntity::Mod(e)) => get_i(&e.nbt, "contain_recipe", 0) != 0,
        _ => false,
    };
    let mut cool = get_i(&entity.nbt, "hz_cooldown", 0);
    if contains && energy(entity) > 0 {
        cool = (cool + 1).min(20);
    } else if cool > 0 {
        cool -= 1;
    }
    set_i(&mut entity.nbt, "hz_cooldown", cool);
    if cool >= 20 {
        let fill = energy(entity).min(transfer);
        let used = orb_fill(game, orb, fill);
        let left = energy(entity) - used;
        set_energy(entity, left);
    }
}

/// A bucket used on a machine's tank: a full one empties into it (a fluid it takes, with room for 1,000 mB), an
/// empty one fills from it. In creative the held item stays as it is. True when it was handled.
fn bucket_click(game: &mut Game<'_>, pos: Pos) -> bool {
    let block = game.native_block(pos);
    let (cap, core) = match machine(&block) {
        Some(Mach::Magmator | Mach::Thermo) => (4000, pos),
        Some(Mach::Reactor) => (1000, reactor_core(game, pos)),
        _ => return false,
    };
    let Some(player) = game.native_acting_player() else { return false };
    let slot = game.selected as i32;
    let Some(held) = game.native_carried_stack(slot).map(|h| h.item.to_string()) else { return false };
    let (fluid, amount) = match game.world.block_entities.get(&(core[0], core[1], core[2])) {
        Some(horizon_client::containers::BlockEntity::Mod(e)) => tank(e),
        _ => return false,
    };
    let core_block = game.native_block(core);
    let creative = player.game_mode == 1;
    let filled_fluid = match held.as_str() {
        "minecraft:water_bucket" => Some("minecraft:water"),
        "minecraft:lava_bucket" => Some("minecraft:lava"),
        _ => None,
    };
    if let Some(f) = filled_fluid {
        let mut refused = false;
        accepts_fluid(&core_block, 0, f, &mut refused);
        if refused || fluid.as_deref().is_some_and(|x| x != f) || amount + 1000 > cap {
            return false;
        }
        game.native_with_mod_block_entity(core, |e| set_tank(e, Some(f), amount + 1000));
        if !creative {
            if let Some(h) = game.native_carried_stack(slot) {
                *h = ItemStack::new("minecraft:bucket", 1);
            }
        }
        return true;
    }
    if held == "minecraft:bucket" && amount >= 1000 {
        let Some(out) = (match fluid.as_deref() {
            Some("minecraft:water") => Some("minecraft:water_bucket"),
            Some("minecraft:lava") => Some("minecraft:lava_bucket"),
            _ => None,
        }) else {
            return false;
        };
        let f = fluid.clone();
        game.native_with_mod_block_entity(core, |e| set_tank(e, f.as_deref(), amount - 1000));
        if !creative {
            let one = game.native_carried_stack(slot).map_or(0, |h| h.count);
            if one <= 1 {
                if let Some(h) = game.native_carried_stack(slot) {
                    *h = ItemStack::new(out, 1);
                }
            } else {
                if let Some(h) = game.native_carried_stack(slot) {
                    h.count -= 1;
                }
                if let Some(rest) = game.native_give_stack(player.uuid, ItemStack::new(out, 1)) {
                    game.spawn_item(rest, player.position, [0.0, 0.0, 0.0], 10);
                }
            }
        }
        return true;
    }
    false
}

/// Where the acting player's look ray first meets the unit cube at `pos` (its eye: 1.62 up, 1.27 sneaking).
fn hit_point(game: &Game<'_>, pos: Pos) -> Option<[f64; 3]> {
    let p = game.native_acting_player()?;
    let eye = [p.position[0], p.position[1] + if game.native_sneaking() { 1.27 } else { 1.62 }, p.position[2]];
    let (y, x) = (f64::from(p.yaw).to_radians(), f64::from(p.pitch).to_radians());
    let dir = [-y.sin() * x.cos(), -x.sin(), y.cos() * x.cos()];
    let (mut t0, mut t1) = (0.0f64, 8.0f64);
    for i in 0..3 {
        let (lo, hi) = (f64::from(pos[i]), f64::from(pos[i]) + 1.0);
        if dir[i].abs() < 1e-9 {
            if eye[i] < lo || eye[i] > hi {
                return None;
            }
            continue;
        }
        let (a, b) = ((lo - eye[i]) / dir[i], (hi - eye[i]) / dir[i]);
        t0 = t0.max(a.min(b));
        t1 = t1.min(a.max(b));
    }
    (t0 <= t1).then(|| [eye[0] + dir[0] * t0, eye[1] + dir[1] * t0, eye[2] + dir[2] * t0])
}

/// CableBlock.getHitSide: the arm a click lands on (outer 0.4 of each side), by the order Java tests them.
fn cable_hit_side(game: &Game<'_>, pos: Pos) -> Option<usize> {
    let h = hit_point(game, pos)?;
    let (x, y, z) = (h[0] - f64::from(pos[0]), h[1] - f64::from(pos[1]), h[2] - f64::from(pos[2]));
    let inner = |v: f64| v > 0.0 && v < 0.4;
    let outer = |v: f64| v > 0.6 && v < 1.0;
    if inner(x) {
        Some(4)
    } else if outer(x) {
        Some(5)
    } else if inner(z) {
        Some(2)
    } else if outer(z) {
        Some(3)
    } else if outer(y) {
        Some(1)
    } else if inner(y) {
        Some(0)
    } else {
        None
    }
}

// ---- The wrench (WrenchItem, WrenchMode: CONFIG, LINK, ROTATE)

const WRENCH: &str = "powah:wrench";
const WRENCH_MODES: [&str; 3] = ["config", "link", "rotate"];

fn wrench_mode(stack: &ItemStack) -> usize {
    match custom(stack).and_then(|c| c.get(&java("WrenchMode"))) {
        Some(Tag::Int(v)) => (*v).clamp(0, 2) as usize,
        _ => 0,
    }
}

fn set_wrench_tag(stack: &mut ItemStack, f: impl FnOnce(&mut Compound)) {
    let mut c = custom(stack).cloned().unwrap_or_default();
    f(&mut c);
    let extra = stack.extra.get_or_insert_with(Default::default);
    extra.other.retain(|(k, _)| k != "minecraft:custom_data");
    extra.other.push(("minecraft:custom_data".into(), Tag::Compound(c)));
}

fn wrench_line(mode: usize) -> String {
    lang(&format!("info.powah.wrench.mode.{}", WRENCH_MODES[mode]), WRENCH_MODES[mode]).replacen("%s", "§e", 1)
}

/// The wrench used on a block (onItemUseFirst): the cable's side config (CONFIG), linking a rod and an orb (LINK).
fn wrench_click(game: &mut Game<'_>, pos: Pos) -> bool {
    let slot = game.selected as i32;
    let Some(held) = game.native_carried_stack(slot).cloned() else { return false };
    if &*held.item != WRENCH || game.native_sneaking() {
        return false;
    }
    let Some(player) = game.native_acting_player() else { return false };
    let block = game.native_block(pos);
    let mode = wrench_mode(&held);
    if mode == 0 && is_cable(&block) {
        if let Some(side) = cable_hit_side(game, pos) {
            game.native_with_mod_block_entity(pos, |e| {
                let mut s = sides(e);
                s[side] = s[side].next(Transfer::All);
                set_sides(e, s);
                fill_cable_data(e);
            });
        }
        return true;
    }
    if mode == 1 && (is_rod(&block) || block == ORB) {
        // EnergizingRodBlock / EnergizingOrbBlock.onWrench: one end remembered on the wrench, then the other links.
        let (mine, other) = if is_rod(&block) { ("RodPos", "OrbPos") } else { ("OrbPos", "RodPos") };
        let stored = match custom(&held).and_then(|c| c.get(&java(other))) {
            Some(Tag::IntArray(a)) if a.len() == 3 => Some([a[0], a[1], a[2]]),
            _ => None,
        };
        let (rod, orb) = match stored {
            Some(o) => if is_rod(&block) { (pos, o) } else { (o, pos) },
            None => {
                if let Some(h) = game.native_carried_stack(slot) {
                    set_wrench_tag(h, |c| {
                        c.insert(java(mine), Tag::IntArray(vec![pos[0], pos[1], pos[2]].into()));
                    });
                }
                let text = lang("chat.powah.wrench.link.start", "Linking...");
                game.native_action_bar(player.uuid, horizon_client::text::Text::legacy(&format!("§e{text}")));
                return true;
            }
        };
        let ok = is_rod(&game.native_block(rod)) && game.native_block(orb) == ORB && {
            let d = (0..3).map(|i| f64::from(rod[i] - orb[i]).powi(2)).sum::<f64>().sqrt();
            d as i32 <= energizing_range()
        };
        if ok {
            game.native_with_mod_block_entity(rod, |e| {
                e.nbt.insert(java("OrbPos"), Tag::IntArray(vec![orb[0], orb[1], orb[2]].into()));
            });
        }
        let (key, english, color) = if ok { ("chat.powah.wrench.link.done", "Linked!", "§6") } else { ("chat.powah.wrench.link.fail", "Too far!", "§c") };
        let text = lang(key, english);
        game.native_action_bar(player.uuid, horizon_client::text::Text::legacy(&format!("{color}{text}")));
        if let Some(h) = game.native_carried_stack(slot) {
            set_wrench_tag(h, |c| {
                c.remove(&java(other));
            });
        }
        return true;
    }
    false
}

/// `right_click_block` on the orb (EnergizingOrbBlock.onBlockActivated): a held item goes in, one, while no result
/// waits; otherwise the last thing in it comes out to the player.
pub(in crate::game) fn right_click(game: &mut Game<'_>, pos: Pos, canceled: &mut bool) {
    if wrench_click(game, pos) {
        *canceled = true;
        return;
    }
    // CableBlock.onBlockActivated: on an arm that reaches energy its screen opens for that side; anywhere else, nothing.
    if is_cable(&game.native_block(pos)) {
        let side = cable_hit_side(game, pos);
        let joined = match game.world.block_entities.get(&(pos[0], pos[1], pos[2])) {
            Some(horizon_client::containers::BlockEntity::Mod(e)) => energy_sides(e),
            _ => [false; 6],
        };
        match side {
            Some(s) if joined[s] => {
                game.native_with_mod_block_entity(pos, |e| {
                    set_i(&mut e.nbt, "hz_menu_side", s as i32);
                    fill_cable_data(e);
                });
                // The cable stands on chorus_plant's template, whose use opens nothing: its menu is opened here (the
                // host's; a LAN guest's menus open in lan_open_container).
                game.open_menu(pos);
                *canceled = true;
            }
            _ => *canceled = true,
        }
        return;
    }
    // MagmatorBlock, ThermoBlock, ReactorBlock: FluidUtil.interactWithFluidHandler, a bucket in or out of the tank.
    if bucket_click(game, pos) {
        *canceled = true;
        return;
    }
    // ReactorBlock.onBlockActivated: a part opens the core's screen (the host's; a LAN guest's menu opens elsewhere).
    if is_reactor(&game.native_block(pos)) {
        let core = reactor_core(game, pos);
        let host = game.native_acting_player().is_some_and(|p| game.serving.as_ref().is_some_and(|s| s.owner == p.uuid));
        if core != pos && host && !game.native_sneaking() {
            *canceled = true;
            game.open_menu(core);
        }
        return;
    }
    if game.native_block(pos) != ORB {
        return;
    }
    let Some(player) = game.native_acting_player() else { return };
    *canceled = true;
    let slot = game.selected as i32;
    let held = game.native_carried_stack(slot).map(|h| h.clone());
    let output_empty = match game.world.block_entities.get(&(pos[0], pos[1], pos[2])) {
        Some(horizon_client::containers::BlockEntity::Mod(e)) => e.items.first().cloned().flatten().is_none(),
        _ => return,
    };
    if let Some(held) = held.filter(|h| h.count > 0 && output_empty) {
        let mut one = held.clone();
        one.count = 1;
        let placed = game
            .native_with_mod_block_entity(pos, |e| {
                if e.items.len() < 7 {
                    e.items.resize(7, None);
                }
                match (1..7).find(|i| e.items[*i].is_none()) {
                    Some(i) => {
                        e.items[i] = Some(one.clone());
                        true
                    }
                    None => false,
                }
            })
            .unwrap_or(false);
        if placed && player.game_mode != 1 {
            if let Some(h) = game.native_carried_stack(slot) {
                h.count -= 1;
            }
        }
    } else {
        let taken = game
            .native_with_mod_block_entity(pos, |e| (0..e.items.len()).rev().find_map(|i| e.items[i].take()))
            .flatten();
        if let Some(stack) = taken {
            if let Some(rest) = game.native_give_stack(player.uuid, stack) {
                game.spawn_item(rest, player.position, [0.0, 0.0, 0.0], 10);
            }
        }
    }
}

// ---- The ender network (EnderNetwork: each owner's 12 channels, saved with the world)

#[derive(Default)]
struct Ender {
    /// Which world's network is loaded (its folder).
    world: Option<std::path::PathBuf>,
    /// (owner, channel) -> (capacity, stored), as Lollipop's longs.
    channels: HashMap<(u128, i32), (i64, i64)>,
    dirty: bool,
}

thread_local! {
    static ENDER: RefCell<Ender> = RefCell::new(Ender::default());
}

fn ender_file(dir: &std::path::Path) -> std::path::PathBuf {
    dir.join("data").join("powah_network.json")
}

/// The network of the world in play, read from its file the first time.
fn with_ender<R>(game: &Game<'_>, f: impl FnOnce(&mut HashMap<(u128, i32), (i64, i64)>, &mut bool) -> R) -> R {
    let dir = game.native_world_dir();
    ENDER.with_borrow_mut(|e| {
        if e.world != dir {
            e.world = dir.clone();
            e.channels.clear();
            e.dirty = false;
            if let Some(text) = dir.as_ref().and_then(|d| std::fs::read_to_string(ender_file(d)).ok()) {
                if let Ok(serde_json::Value::Array(rows)) = serde_json::from_str::<serde_json::Value>(&text) {
                    for r in rows {
                        let owner = r["owner"].as_str().and_then(|s| u128::from_str_radix(s, 16).ok());
                        if let (Some(owner), Some(ch)) = (owner, r["channel"].as_i64()) {
                            e.channels.insert((owner, ch as i32), (r["capacity"].as_i64().unwrap_or(0), r["stored"].as_i64().unwrap_or(0)));
                        }
                    }
                }
            }
        }
        let Ender { channels, dirty, .. } = e;
        f(channels, dirty)
    })
}

/// `world_saved`: the network goes to data/powah_network.json (WorldSavedData "powah_network").
pub(in crate::game) fn world_saved(game: &mut Game<'_>) {
    let Some(dir) = game.native_world_dir() else { return };
    ENDER.with_borrow_mut(|e| {
        if !e.dirty || e.world.as_ref() != Some(&dir) {
            return;
        }
        let rows: Vec<serde_json::Value> = e
            .channels
            .iter()
            .map(|((owner, ch), (cap, stored))| serde_json::json!({ "owner": format!("{owner:032x}"), "channel": ch, "capacity": cap, "stored": stored }))
            .collect();
        let file = ender_file(&dir);
        let _ = file.parent().map(std::fs::create_dir_all);
        if std::fs::write(&file, serde_json::to_string_pretty(&rows).unwrap_or_default()).is_ok() {
            e.dirty = false;
        }
    });
}

fn owner(entity: &ModBlockEntity) -> Option<u128> {
    match entity.nbt.get(&java("owner_id")) {
        Some(Tag::IntArray(a)) if a.len() == 4 => Some(a.iter().fold(0u128, |acc, v| (acc << 32) | u128::from(*v as u32))),
        _ => None,
    }
}

fn channel(entity: &ModBlockEntity) -> i32 {
    get_i(&entity.nbt, "channel", 0).clamp(0, 11)
}

fn max_channels(entity: &ModBlockEntity) -> i32 {
    let Some((kind, Some(tier))) = kind_tier(&entity.id) else { return 0 };
    config(kind).get(&format!("Channels.{}Channels", TIERS[tier])).copied().unwrap_or(1).clamp(1, 12) as i32
}

/// The block's buffer shows its channel's energy: what other blocks put in or took out of the buffer since its
/// last tick goes to the network first (within the network's capacity), then the buffer takes the network's amount.
fn ender_sync(game: &Game<'_>, entity: &mut ModBlockEntity) {
    let Some(id) = owner(entity) else {
        set_i(&mut entity.nbt, "energy", 0);
        set_i(&mut entity.nbt, "hz_shown", 0);
        return;
    };
    let ch = channel(entity);
    let shown = get_i(&entity.nbt, "hz_shown", 0);
    let delta = i64::from(energy(entity)) - i64::from(shown);
    let (cap, stored) = with_ender(game, |n, dirty| {
        let e = n.entry((id, ch)).or_insert((0, 0));
        if delta != 0 {
            e.1 = (e.1 + delta).clamp(0, e.0);
            *dirty = true;
        }
        *e
    });
    let now = stored.clamp(0, i64::from(i32::MAX)) as i32;
    set_i(&mut entity.nbt, "energy", now);
    set_i(&mut entity.nbt, "hz_shown", now);
    put_wide_nbt(entity, cap);
}

/// The network's capacity, for the screen.
fn put_wide_nbt(entity: &mut ModBlockEntity, cap: i64) {
    set_i(&mut entity.nbt, "hz_net_cap", cap.clamp(0, i64::from(i32::MAX)) as i32);
}

/// onSlotChanged(0): an energy cell put in slot 0 adds its capacity and energy to the channel, and is used up.
fn ender_extend(game: &mut Game<'_>, entity: &mut ModBlockEntity) {
    let Some(id) = owner(entity) else { return };
    let Some(stack) = entity.items.first().cloned().flatten() else { return };
    // IEnderExtender: an energy cell or a battery.
    let add_cap = if is_battery(&stack.item) {
        item_energy(&stack).map_or(0, |e| e.1)
    } else if is_cell(&stack.item) && !is_creative(&stack.item) {
        energy_of(&stack.item).map_or(0, |e| e.0)
    } else {
        return;
    };
    let add_energy = game.native_item_energy(&stack).map_or(0, |e| e.amount);
    let ch = channel(entity);
    let ok = with_ender(game, |n, dirty| {
        let e = n.entry((id, ch)).or_insert((0, 0));
        let new_cap = e.0 + i64::from(add_cap);
        if add_cap > 0 && new_cap <= i64::MAX / 2 {
            e.0 = new_cap;
            e.1 = (e.1 + i64::from(add_energy)).min(e.0);
            *dirty = true;
            true
        } else {
            false
        }
    });
    if ok {
        entity.items[0] = if stack.count > 1 { Some(ItemStack { count: stack.count - 1, ..stack }) } else { None };
        let shown = with_ender(game, |n, _| n.get(&(id, ch)).map_or(0, |e| e.1));
        let now = shown.clamp(0, i64::from(i32::MAX)) as i32;
        set_i(&mut entity.nbt, "energy", now);
        set_i(&mut entity.nbt, "hz_shown", now);
    }
}

// ---- The reactor (ReactorTile, Builder, ReactorPartTile, ReactorItem)

/// The 3x4x3 around the core, the core left out (Builder.getPosList).
fn reactor_parts(core: Pos) -> Vec<Pos> {
    let mut out = Vec::new();
    for y in 0..4 {
        for x in -1..=1 {
            for z in -1..=1 {
                if (x, y, z) != (0, 0, 0) {
                    out.push([core[0] + x, core[1] + y, core[2] + z]);
                }
            }
        }
    }
    out
}

/// Builder.isDone: one part placed every 5 ticks, in a shuffled order, then the structure is built.
fn reactor_build(game: &mut Game<'_>, entity: &mut ModBlockEntity, core: Pos) -> bool {
    if get_i(&entity.nbt, "built", 0) != 0 {
        return true;
    }
    let mut queue: Vec<i32> = match entity.nbt.get(&java("queue_pos")) {
        Some(Tag::IntArray(a)) => a.to_vec(),
        _ => Vec::new(),
    };
    let delay = get_i(&entity.nbt, "hz_delay", 5) - 1;
    if !queue.is_empty() {
        if delay <= 0 && queue.len() >= 3 {
            let p = [queue[0], queue[1], queue[2]];
            queue.drain(0..3);
            let block = entity.id.clone();
            let dim = game.native_dimension().to_owned();
            if let Some(part) = game.native_default_state(&block) {
                game.native_set_block(&dim, p, part, 3);
            }
            game.native_with_mod_block_entity(p, |e| {
                set_i(&mut e.nbt, "hz_part", 1);
                e.nbt.insert(java("core_pos"), Tag::IntArray(vec![core[0], core[1], core[2]].into()));
            });
            set_i(&mut entity.nbt, "hz_delay", 5);
        } else {
            set_i(&mut entity.nbt, "hz_delay", delay);
        }
        entity.nbt.insert(java("queue_pos"), Tag::IntArray(queue.into()));
        return false;
    }
    set_i(&mut entity.nbt, "built", 1);
    true
}

/// Builder.isDone's extractors: the part beside the core on each side but below, and the top's middle (core + up 3).
fn reactor_extractors(core: Pos) -> [Pos; 5] {
    [[core[0], core[1], core[2] - 1], [core[0], core[1], core[2] + 1], [core[0] - 1, core[1], core[2]], [core[0] + 1, core[1], core[2]], [core[0], core[1] + 3, core[2]]]
}

/// ReactorPartTile.getCapability: a built part is the core to the outside. Items and fluids put into a part (a hopper,
/// a pipe) go on to the core's slots and tank; energy taken from an extractor came out of the core.
fn reactor_parts_forward(game: &mut Game<'_>, core_entity: &mut ModBlockEntity, core: Pos) {
    for p in reactor_parts(core) {
        let (items, fluid, drained) = match game.world.block_entities.get(&(p[0], p[1], p[2])) {
            Some(horizon_client::containers::BlockEntity::Mod(e)) if get_i(&e.nbt, "hz_part", 0) != 0 => {
                let items = e.items.iter().any(Option::is_some);
                let fluid = tank(e).1 > 0;
                let drained = get_i(&e.nbt, "hz_shown", 0) - energy(e);
                (items, fluid, drained)
            }
            _ => continue,
        };
        if drained > 0 {
            let left = energy(core_entity) - drained;
            set_energy(core_entity, left);
        }
        if !items && !fluid {
            continue;
        }
        let moved = game.native_with_mod_block_entity(p, |e| {
            let items: Vec<Option<ItemStack>> = std::mem::take(&mut e.items);
            let (f, a) = tank(e);
            set_tank(e, None, 0);
            (items, f, a)
        });
        let Some((items, f, a)) = moved else { continue };
        let mut back: Vec<Option<ItemStack>> = vec![None; items.len()];
        for (slot, stack) in items.into_iter().enumerate() {
            let Some(stack) = stack else { continue };
            // Into the core's same slot when it holds the same item or nothing (the slot rules are the core's).
            let fits = match core_entity.items.get(slot).cloned().flatten() {
                None => true,
                Some(have) => have.item == stack.item && have.count + stack.count <= 64,
            };
            if fits && slot < core_entity.items.len() {
                let now = match core_entity.items[slot].take() {
                    Some(mut have) => {
                        have.count += stack.count;
                        have
                    }
                    None => stack,
                };
                core_entity.items[slot] = Some(now);
            } else {
                back[slot] = Some(stack);
            }
        }
        let (cf, ca) = tank(core_entity);
        let mut fluid_back = 0;
        if let Some(f) = f.as_deref() {
            if cf.as_deref().is_none_or(|x| x == f) && coolant(f).is_some() {
                let room = (1000 - ca).max(0);
                let take = a.min(room);
                set_tank(core_entity, Some(f), ca + take);
                fluid_back = a - take;
            } else {
                fluid_back = a;
            }
        }
        if back.iter().any(Option::is_some) || fluid_back > 0 {
            let fl = f.clone();
            game.native_with_mod_block_entity(p, |e| {
                e.items = back;
                set_tank(e, fl.as_deref(), fluid_back);
            });
        }
    }
}

/// The extractors show the core's energy (and give it, through `reactor_parts_forward`); the other parts none.
fn reactor_parts_mirror(game: &mut Game<'_>, core_entity: &ModBlockEntity, core: Pos) {
    let e = energy(core_entity);
    for p in reactor_extractors(core) {
        game.native_with_mod_block_entity(p, |part| {
            if get_i(&part.nbt, "hz_part", 0) != 0 {
                set_i(&mut part.nbt, "energy", e);
                set_i(&mut part.nbt, "hz_shown", e);
            }
        });
    }
}

/// PowahAPI's solid coolants (Powah.registerSolidCoolant): item -> (size, temperature).
fn solid_coolant(item: &str) -> Option<(i32, i32)> {
    Some(match item {
        "minecraft:snow_block" => (48, -3),
        "minecraft:snowball" => (12, -3),
        "minecraft:ice" => (48, -5),
        "minecraft:packed_ice" => (192, -8),
        "minecraft:blue_ice" => (568, -17),
        "powah:dry_ice" => (712, -32),
        _ => return None,
    })
}

/// A Ticker kept as hundredths in the entity: (ticks, max).
fn ticker(entity: &ModBlockEntity, key: &str) -> (f64, f64) {
    (f64::from(get_i(&entity.nbt, key, 0)) / 100.0, f64::from(get_i(&entity.nbt, &format!("{key}_max"), 0)) / 100.0)
}

fn set_ticker(entity: &mut ModBlockEntity, key: &str, ticks: f64, max: f64) {
    set_i(&mut entity.nbt, key, (ticks * 100.0).round() as i32);
    set_i(&mut entity.nbt, &format!("{key}_max"), (max * 100.0).round() as i32);
}

fn take_one(entity: &mut ModBlockEntity, slot: usize) {
    if let Some(Some(s)) = entity.items.get_mut(slot) {
        if s.count > 1 {
            s.count -= 1;
        } else {
            entity.items[slot] = None;
        }
    }
}

/// ReactorTile.postTick: fuel, carbon, redstone, temperature, the energy made, generation mode, and the energy given
/// out through the parts two blocks to each side, four above and one below.
fn reactor_tick(game: &mut Game<'_>, entity: &mut ModBlockEntity, pos: Pos, cap: i32, transfer: i32, powered: bool) {
    let Some((_, Some(tier))) = kind_tier(&entity.id) else { return };
    if entity.items.len() < 5 {
        entity.items.resize(5, None);
    }
    let ticks = get_i(&entity.nbt, "hz_ticks", 0).wrapping_add(1);
    set_i(&mut entity.nbt, "hz_ticks", ticks);
    charge_items_from(game, entity, 0, 1, transfer);
    let (mut fuel, _) = ticker(entity, "fuel");
    let (mut carbon, mut carbon_max) = ticker(entity, "carbon");
    let (mut redstone, mut redstone_max) = ticker(entity, "redstone");
    let (mut solid, mut solid_max) = ticker(entity, "solid_coolant");
    let (mut temp, _) = ticker(entity, "temperature");
    let mut base_temp = get_i(&entity.nbt, "base_temp", 0);
    let mut carbon_temp = get_i(&entity.nbt, "carbon_temp", 0);
    let mut redstone_temp = get_i(&entity.nbt, "redstone_temp", 0);
    let mut solid_temp = get_i(&entity.nbt, "solid_coolant_temp", 0);
    let generate_on = get_i(&entity.nbt, "generate", 1) != 0;
    let running_before = get_i(&entity.nbt, "running", 0) != 0;
    let mut running = false;
    let item = |e: &ModBlockEntity, i: usize| e.items.get(i).cloned().flatten();
    if powered && generate_on {
        let stored = energy(entity);
        let generating = stored < cap && fuel > 0.0;
        // processFuel: a uraninite adds 100 while there's room for it.
        if fuel <= 900.0 && item(entity, 1).is_some_and(|s| &*s.item == "powah:uraninite") {
            fuel += 100.0;
            base_temp = 700;
            take_one(entity, 1);
        }
        if fuel <= 0.0 {
            base_temp = 0;
        }
        // processCarbon: a fuel's burn time, 180 more heat.
        if carbon <= 0.0 {
            if let Some(s) = item(entity, 2) {
                let burn = game.res.providers().burn_time(game.res.items(), &s, false) as f64;
                if burn > 0.0 {
                    carbon = burn;
                    carbon_max = burn;
                    carbon_temp = 180;
                    take_one(entity, 2);
                }
            }
        }
        if carbon > 0.0 {
            if generating {
                carbon -= 1.0;
                if carbon <= 0.0 {
                    carbon = 0.0;
                    carbon_max = 0.0;
                }
            }
        } else {
            carbon_temp = 0;
        }
        // processRedstone: dust 18, a block 162; 120 more heat.
        if redstone <= 0.0 {
            match item(entity, 3).map(|s| s.item.to_string()).as_deref() {
                Some("minecraft:redstone") => {
                    redstone = 18.0;
                    redstone_max = 18.0;
                    redstone_temp = 120;
                    take_one(entity, 3);
                }
                Some("minecraft:redstone_block") => {
                    redstone = 162.0;
                    redstone_max = 162.0;
                    redstone_temp = 120;
                    take_one(entity, 3);
                }
                _ => {}
            }
        }
        if redstone > 0.0 {
            if generating && ticks % 40 == 0 {
                redstone -= 1.0;
                if redstone <= 0.0 {
                    redstone = 0.0;
                    redstone_max = 0.0;
                }
            }
        } else {
            redstone_temp = 0;
        }
        // processTemperature: solid coolant, then the temperature it settles at, with the tank's coolant.
        let (fluid, amount) = tank(entity);
        if solid <= 0.0 {
            if let Some((size, t)) = item(entity, 4).and_then(|s| solid_coolant(&s.item)) {
                if size > 0 && t < 2 {
                    solid = f64::from(size);
                    solid_max = f64::from(size);
                    solid_temp = t;
                    take_one(entity, 4);
                }
            }
        }
        if solid > 0.0 {
            if amount > 0 && generating && ticks % 40 == 0 {
                solid -= 1.0;
                if solid <= 0.0 {
                    solid = 0.0;
                    solid_max = 0.0;
                }
            }
        } else {
            solid_temp = 0;
        }
        let mut target = f64::from(base_temp + carbon_temp + redstone_temp).min(1000.0);
        if amount > 0 {
            let coldness = fluid.as_deref().and_then(coolant).unwrap_or(1) - 2;
            target /= f64::from((coldness + solid_temp).abs() + 1);
        }
        if temp < target {
            temp += 1.0;
        }
        let every = if amount == 0 { 5 } else if solid <= 0.0 { 3 } else { 1 };
        if ticks % every == 0 && temp > target {
            temp -= 1.0;
        }
        temp = temp.clamp(0.0, 1000.0);
        if generating {
            // calc, calcConsumption, calcProduction.
            let calc = temp / 1000.0 * 0.98 / 2.0 * if redstone <= 0.0 { 1.0 } else { 1.4 };
            let consumption = if running_before { (1.0 + tier as f64 * 0.25) * calc } else { 0.0 };
            fuel = (fuel - consumption).max(0.0);
            let production = (1.0 - calc) * (fuel / 100.0) * f64::from(generation_of(&entity.id)) * if carbon <= 0.0 { 1.0 } else { 1.2 } * if redstone <= 0.0 { 1.0 } else { 1.4 };
            let now = (i64::from(energy(entity)) + production as i64).min(i64::from(cap)) as i32;
            set_energy(entity, now);
            running = true;
        }
    }
    // checkGenMode: with the mode on, it stops when full and starts again at 70%.
    if get_i(&entity.nbt, "gen_mode", 0) != 0 {
        let e = energy(entity);
        if e >= cap {
            set_i(&mut entity.nbt, "generate", 0);
        } else if i64::from(e) * 100 / i64::from(cap.max(1)) <= 70 {
            set_i(&mut entity.nbt, "generate", 1);
        }
    }
    set_ticker(entity, "fuel", fuel, 1000.0);
    set_ticker(entity, "carbon", carbon, carbon_max);
    set_ticker(entity, "redstone", redstone, redstone_max);
    set_ticker(entity, "solid_coolant", solid, solid_max);
    set_ticker(entity, "temperature", temp, 1000.0);
    set_i(&mut entity.nbt, "base_temp", base_temp);
    set_i(&mut entity.nbt, "carbon_temp", carbon_temp);
    set_i(&mut entity.nbt, "redstone_temp", redstone_temp);
    set_i(&mut entity.nbt, "solid_coolant_temp", solid_temp);
    set_i(&mut entity.nbt, "running", i32::from(running));
    // The energy out: past the parts, by each side set to give.
    let s = sides(entity);
    for d in 0..6 {
        if !s[d].extracts() {
            continue;
        }
        let have = energy(entity).min(transfer);
        if have <= 0 {
            break;
        }
        let reach = match d {
            0 => 1,
            1 => 4,
            _ => 2,
        };
        let step_d = [[0, -1, 0], [0, 1, 0], [0, 0, -1], [0, 0, 1], [-1, 0, 0], [1, 0, 0]][d];
        let there = [pos[0] + step_d[0] * reach, pos[1] + step_d[1] * reach, pos[2] + step_d[2] * reach];
        let moved = if is_cable(&game.native_block(there)) {
            cable_receive(game, there, opposite(d), have, false)
        } else {
            game.native_energy_insert(there, FACES[opposite(d)], have, false)
        };
        let left = energy(entity) - moved;
        set_energy(entity, left);
    }
}

/// ReactorItem.onItemUse and Builder.shuffle: the core needs the whole 3x4x3 free and 36 reactors (35 more are
/// taken); its parts are queued in a shuffled order. Refused: the block is taken back and the item returned.
fn reactor_placed(game: &mut Game<'_>, stack: &ItemStack, pos: Pos) {
    let Some(player) = game.native_acting_player() else { return };
    let free = |b: &str| matches!(b, "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air" | "minecraft:water" | "minecraft:lava" | "minecraft:short_grass" | "minecraft:tall_grass" | "minecraft:fern" | "minecraft:large_fern" | "minecraft:snow" | "minecraft:vine" | "minecraft:dead_bush" | "minecraft:seagrass");
    let room = reactor_parts(pos).into_iter().all(|p| free(&game.native_block(p)));
    let creative = player.game_mode == 1;
    let slot = game.selected as i32;
    let left = game.native_carried_stack(slot).map_or(0, |h| if &*h.item == &*stack.item { h.count } else { 0 });
    if !room || (!creative && left < 35) {
        remove_later(game, pos, &stack.item);
        if !creative {
            if let Some(h) = game.native_carried_stack(slot) {
                h.count += 1;
            }
        }
        if room {
            let text = lang("chat.powah.not.enough.blocks", "Not enough blocks, need %s more!").replacen("%s", &format!("§e{}§c", 36 - (left + 1)), 1);
            game.native_action_bar(player.uuid, horizon_client::text::Text::legacy(&format!("§c{text}")));
        }
        return;
    }
    if !creative {
        if let Some(h) = game.native_carried_stack(slot) {
            h.count -= 35;
        }
    }
    let mut parts = reactor_parts(pos);
    // Collections.shuffle, from the world's random.
    for i in (1..parts.len()).rev() {
        let dim = game.native_dimension().to_owned();
        let j = game
            .native_level_random(&dim)
            .and_then(|r| {
                use horizon_core::random::RandomSource;
                r.next_int_bound((i + 1) as i32).ok()
            })
            .unwrap_or(0) as usize;
        parts.swap(i, j);
    }
    let flat: Vec<i32> = parts.iter().flat_map(|p| p.iter().copied()).collect();
    game.native_with_mod_block_entity(pos, |e| {
        e.nbt.insert(java("queue_pos"), Tag::IntArray(flat.clone().into()));
        set_i(&mut e.nbt, "built", 0);
        set_i(&mut e.nbt, "hz_delay", 5);
    });
}

/// ReactorBlock.onReplaced and Builder.demolish: breaking any of it takes all of it down; the core drops one reactor
/// for each block still standing and each still queued, and its uraninite.
fn reactor_broken(game: &mut Game<'_>, pos: Pos, block: &str, drops: &mut Vec<ItemStack>) {
    drops.retain(|d| &*d.item != block);
    drops.extend(reactor_demolish(game, pos, block));
}

/// `destroy_block`: a player breaking any of it (onReplaced, whatever the tool) brings it down, its items at the core.
pub(in crate::game) fn destroyed(game: &mut Game<'_>, pos: Pos) {
    let block = game.native_block(pos);
    if !is_reactor(&block) {
        return;
    }
    let core = reactor_core(game, pos);
    for stack in reactor_demolish(game, pos, &block) {
        game.spawn_item(stack, [f64::from(core[0]) + 0.5, f64::from(core[1]) + 0.5, f64::from(core[2]) + 0.5], [0.0, 0.0, 0.0], 10);
    }
    STATE.with_borrow_mut(|s| {
        s.demolished.insert(pos);
    });
}

fn reactor_core(game: &Game<'_>, pos: Pos) -> Pos {
    match game.world.block_entities.get(&(pos[0], pos[1], pos[2])) {
        Some(horizon_client::containers::BlockEntity::Mod(e)) if get_i(&e.nbt, "hz_part", 0) != 0 => match e.nbt.get(&java("core_pos")) {
            Some(Tag::IntArray(a)) if a.len() == 3 => [a[0], a[1], a[2]],
            _ => pos,
        },
        _ => pos,
    }
}

/// Builder.demolish: the other blocks queued away; one reactor for each block standing and each still queued, and
/// a uraninite for each 100 of fuel.
fn reactor_demolish(game: &mut Game<'_>, pos: Pos, block: &str) -> Vec<ItemStack> {
    let core = reactor_core(game, pos);
    let (queued, fuel) = match game.world.block_entities.get(&(core[0], core[1], core[2])) {
        Some(horizon_client::containers::BlockEntity::Mod(e)) => (
            match e.nbt.get(&java("queue_pos")) {
                Some(Tag::IntArray(a)) => a.len() / 3,
                _ => 0,
            },
            ticker(e, "fuel").0,
        ),
        _ => (0, 0.0),
    };
    let mut count = 0u32;
    for p in reactor_parts(core).into_iter().chain(std::iter::once(core)) {
        if p != pos && game.native_block(p) == block {
            remove_later(game, p, block);
            count += 1;
        }
    }
    let mut out = vec![ItemStack::new(block, count + 1 + queued as u32)];
    let mut f = fuel;
    while f >= 100.0 {
        out.push(ItemStack::new("powah:uraninite", 1));
        f -= 100.0;
    }
    out
}

/// `inventory_tick`: a charging battery charges the player's other energy items (Energy.chargeInventory), each up to
/// its own transfer; batteries are left out.
pub(in crate::game) fn inventory_tick(game: &mut Game<'_>, slot: i32, _selected: bool) {
    let Some(battery) = game.native_carried_stack(slot).cloned() else { return };
    if !is_battery(&battery.item) || !charging(&battery) || game.native_acting_player().is_none() {
        return;
    }
    let Some((_, _, io)) = item_energy(&battery) else { return };
    let mut battery = battery;
    for other in 0..41 {
        if other == slot {
            continue;
        }
        let Some(target) = game.native_carried_stack(other).cloned() else { continue };
        if is_battery(&target.item) || game.native_item_energy(&target).is_none() {
            continue;
        }
        let have = game.native_item_energy(&battery).map_or(0, |e| e.amount);
        let amount = io.min(have);
        if amount <= 0 {
            break;
        }
        let Some((moved, charged)) = game.native_item_energy_insert(&target, amount) else { continue };
        if moved <= 0 {
            continue;
        }
        let Some((_, drained)) = game.native_item_energy_extract(&battery, moved) else { continue };
        battery = drained;
        if let Some(t) = game.native_carried_stack(other) {
            *t = charged;
        }
    }
    if let Some(b) = game.native_carried_stack(slot) {
        *b = battery;
    }
}


// ---- What clients draw (client/render/tile: CableRenderer, ReactorRenderer, ReactorPartRenderer, EnergizingOrbRenderer,
// EnergizingRodRenderer, FurnatorRenderer, MagmatorRenderer; client/model: CableModel, ReactorModel, OrbModel and
// Lollipop's CubeModel). The server sends each block what its renderer reads (`update_tag`) and sends it again when
// that changes (`native_block_entity_sync`).

use horizon_client::mobs::model_layers::{CubeListBuilder, LayerDefinition, MeshDefinition, ModelPart, PartPose};

/// CableModel's arms and plates: (name, texOffs, box origin, box size), each about (0, 14, 0), on a 64x32 texture.
const CABLE_PARTS: [(&str, [i32; 2], [f32; 3], [f32; 3]); 12] = [
    ("north", [0, 10], [-1.5, -1.5, -7.75], [3.0, 3.0, 6.0]),
    ("north_plate", [0, 20], [-2.5, -2.5, -8.2], [5.0, 5.0, 1.0]),
    ("south", [0, 0], [-1.5, -1.5, 1.75], [3.0, 3.0, 6.0]),
    ("south_plate", [0, 20], [-2.5, -2.5, 7.2], [5.0, 5.0, 1.0]),
    ("west", [19, 0], [-7.75, -1.5, -1.5], [6.0, 3.0, 3.0]),
    ("west_plate", [13, 20], [-8.2, -2.5, -2.5], [1.0, 5.0, 5.0]),
    ("east", [19, 7], [1.75, -1.5, -1.5], [6.0, 3.0, 3.0]),
    ("east_plate", [13, 20], [7.2, -2.5, -2.5], [1.0, 5.0, 5.0]),
    ("down", [38, 0], [-1.5, -7.75, -1.5], [3.0, 6.0, 3.0]),
    ("up_plate", [26, 20], [-2.5, 7.2, -2.5], [5.0, 1.0, 5.0]),
    ("up", [38, 10], [-1.5, 1.75, -1.5], [3.0, 6.0, 3.0]),
    ("down_plate", [26, 20], [-2.5, -8.2, -2.5], [5.0, 1.0, 5.0]),
];

fn one_box(tex: [i32; 2], origin: [f32; 3], size: [f32; 3], pivot: [f32; 3], w: u32, h: u32) -> LayerDefinition {
    let mut mesh = MeshDefinition::new();
    let root = mesh.root();
    mesh.add_or_replace_child(root, "part", CubeListBuilder::create().tex_offs(tex[0], tex[1]).add_box(origin, size), PartPose::offset(pivot[0], pivot[1], pivot[2]));
    LayerDefinition::create(mesh, w, h)
}

/// `layer_definitions`: the models above, each box its own layer (`powah:cable#north`, `powah:reactor#main`, ...).
pub fn layer_definitions(layers: &mut Vec<(String, LayerDefinition)>) {
    for (name, tex, origin, size) in CABLE_PARTS {
        layers.push((format!("powah:cable#{name}"), one_box(tex, origin, size, [0.0, 14.0, 0.0], 64, 32)));
    }
    layers.push(("powah:reactor#main".to_owned(), one_box([0, 0], [-24.0, -32.0, -24.0], [48.0, 64.0, 48.0], [0.0, -8.0, 0.0], 256, 128)));
    layers.push(("powah:orb#main".to_owned(), one_box([0, 0], [-2.5, -2.5, -2.5], [5.0, 5.0, 5.0], [0.0, 0.0, 0.0], 20, 10)));
    layers.push(("powah:cube#main".to_owned(), one_box([0, 0], [-8.0, -8.0, -8.0], [16.0, 16.0, 16.0], [0.0, 0.0, 0.0], 64, 32)));
}

thread_local! {
    static BAKED: RefCell<HashMap<String, ModelPart>> = RefCell::new(HashMap::new());
}

fn draw_layer(game: &Game<'_>, draw: &mut crate::game::native_block_entity_draw::BlockEntityDraw, layer: &str, sprite: &str, tint: u32) {
    let part = BAKED.with_borrow_mut(|b| {
        if !b.contains_key(layer) {
            if let Some(p) = game.native_bake_layer(layer) {
                b.insert(layer.to_owned(), p);
            }
        }
        b.get(layer).cloned()
    });
    if let Some(p) = part {
        game.native_draw_model_part(draw, &p, sprite, tint);
    }
}

/// Lollipop's tiers' colours (Tier.color), for the rods' beams.
const TIER_COLORS: [u32; 7] = [10_987_431, 10_726_303, 12_298_643, 14_987_328, 1_306_322, 11_526_721, 14_120_044];

/// The rods and the orb aim at each other's middle (EnergizingOrbTile.getOrbCenter: 0.1 towards its up).
fn orb_up(game: &Game<'_>, state: horizon_world::registry::BlockStateId) -> usize {
    // getOrbUp: the facing's opposite; UP without one.
    game.native_block_state_property(state, "facing").and_then(|f| FACE_NAMES.iter().position(|n| *n == f)).map_or(1, opposite)
}

/// `block_entity_submit`.
pub(in crate::game) fn submit(game: &Game<'_>, pos: Pos, block: &str, state: horizon_world::registry::BlockStateId, entity: &ModBlockEntity, draw: &mut crate::game::native_block_entity_draw::BlockEntityDraw) {
    if is_cable(block) {
        // CableModel.render: on each side reaching a machine and set to give or take, the arm and the plate, in the
        // texture of its config (all, out for RECEIVE, in for EXTRACT); the model is drawn upside down.
        let joined = energy_sides(entity);
        let s = sides(entity);
        draw.push();
        draw.translate([0.5, 1.5, 0.5]);
        draw.translate([0.0, -0.125, 0.0]);
        draw.scale([1.0, -1.0, -1.0]);
        // flags by Direction index -> the parts drawn (the model's y and z are flipped).
        const DRAWN: [(&str, &str); 6] = [("up", "up_plate"), ("down", "down_plate"), ("south", "south_plate"), ("north", "north_plate"), ("west", "west_plate"), ("east", "east_plate")];
        for d in 0..6 {
            if !joined[d] || s[d] == Transfer::None {
                continue;
            }
            let tex = match s[d] {
                Transfer::All => "powah:block/tile/energy_cable_all",
                Transfer::Receive => "powah:block/tile/energy_cable_out",
                _ => "powah:block/tile/energy_cable_in",
            };
            draw_layer(game, draw, &format!("powah:cable#{}", DRAWN[d].0), tex, 0xFF_FFFF);
            draw_layer(game, draw, &format!("powah:cable#{}", DRAWN[d].1), tex, 0xFF_FFFF);
        }
        draw.pop();
        return;
    }
    if is_reactor(block) {
        let tier = kind_tier(block).and_then(|(_, t)| t).unwrap_or(0);
        let built = get_i(&entity.nbt, "built", 0) != 0;
        let part = get_i(&entity.nbt, "hz_part", 0) != 0;
        draw.push();
        draw.translate([0.5, 0.5, 0.5]);
        draw.scale([1.0, -1.0, -1.0]);
        if !built {
            // Not built yet (and every part until then): Lollipop's 16-pixel cube in reactor_block_<tier>.
            draw_layer(game, draw, "powah:cube#main", &format!("powah:block/tile/reactor_block_{}", TIERS[tier]), 0xFF_FFFF);
        } else if !part {
            // ReactorModel: the 3x4x3 shell, then lit "on" while running, "filled" while it holds fuel, and the tier's mark.
            draw.translate([0.0, -1.0, 0.0]);
            draw_layer(game, draw, "powah:reactor#main", "powah:block/tile/reactor", 0xFF_FFFF);
            let light = draw.light;
            if get_i(&entity.nbt, "running", 0) != 0 {
                draw.light = 0x00F0_00F0;
                draw_layer(game, draw, "powah:reactor#main", "powah:block/tile/reactor_on", 0xFF_FFFF);
                draw.light = light;
            }
            if get_i(&entity.nbt, "hz_fueled", 0) != 0 {
                draw.light = 0x00F0_00F0;
                draw_layer(game, draw, "powah:reactor#main", "powah:block/tile/reactor_filled", 0xFF_FFFF);
                draw.light = light;
            }
            if tier != 0 {
                draw_layer(game, draw, "powah:reactor#main", &format!("powah:block/tile/reactor_{}", TIERS[tier]), 0xFF_FFFF);
            }
        }
        draw.pop();
        return;
    }
    if block == ORB {
        // EnergizingOrbRenderer: the result alone, spinning, or the ingredients round a small circle, 0.35 big; then
        // OrbModel (energy_charge) 1.8 times, 0.1 towards the orb's up.
        let turn = -((draw.game_time as f64 + f64::from(draw.partial)) / 200.0 * 360.0) as f32;
        let items: Vec<String> = match entity.nbt.get(&java("hz_items")) {
            Some(Tag::List(l)) => l.iter().filter_map(|t| match t {
                Tag::String(s) => Some(s.to_string_lossy().to_string()),
                _ => None,
            }).collect(),
            _ => Vec::new(),
        };
        let result = get_i(&entity.nbt, "hz_result", 0) != 0;
        let n = items.len();
        for (i, id) in items.iter().enumerate() {
            if id.is_empty() {
                continue;
            }
            draw.push();
            if result || n == 1 {
                draw.translate([0.5, 0.6, 0.5]);
            } else {
                // V3d.circled(n, 0.1): evenly round a circle of radius 0.1 in x-z.
                let a = std::f32::consts::TAU * i as f32 / n as f32;
                draw.translate([0.1 * a.cos() + 0.5, 0.6, 0.1 * a.sin() + 0.5]);
            }
            draw.scale([0.35, 0.35, 0.35]);
            draw.rotate_about([0.0, 1.0, 0.0], turn, [0.0, 0.0, 0.0]);
            game.native_draw_item(draw, &ItemStack::new(id, 1), horizon_assets::item_model::DisplayContext::Fixed, None, i as i32);
            draw.pop();
        }
        // OrbModel (energy_charge, 1.8x, towards the orb's up) is drawn additively in Java (RenderTypes.entityBlendedNoDept:
        // SRC_ALPHA, ONE, no depth test): its dark texture adds a faint glow. Horizon's block-entity draws have no
        // additive layer, and drawn opaque it would hide the items, so it is left out (a gap noted in PLAN).
        let _ = orb_up(game, state);
        return;
    }
    if is_rod(block) {
        // EnergizingRodRenderer: a beam to its orb while it pours (warmed up) or while the player holds a wrench in
        // LINK mode; two crossed strips 0.24 wide in the tier's colour, full bright.
        let Some(Tag::IntArray(o)) = entity.nbt.get(&java("OrbPos")) else { return };
        if o.len() != 3 {
            return;
        }
        let linking = game.held().is_some_and(|h| &*h.item == WRENCH && wrench_mode(h) == 1);
        if get_i(&entity.nbt, "hz_cooldown", 0) < 20 && !linking {
            return;
        }
        let tier = kind_tier(block).and_then(|(_, t)| t).unwrap_or(0);
        let start = [0.5f32, 0.5, 0.5];
        let end = [(o[0] - pos[0]) as f32 + 0.5, (o[1] - pos[1]) as f32 + 0.6, (o[2] - pos[2]) as f32 + 0.5];
        let dir = [end[0] - start[0], end[1] - start[1], end[2] - start[2]];
        let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt().max(1e-4);
        let n = [dir[0] / len, dir[1] / len, dir[2] / len];
        let helper = if n[1].abs() < 0.9 { [0.0, 1.0, 0.0] } else { [1.0, 0.0, 0.0] };
        let cross = |a: [f32; 3], b: [f32; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
        let p1 = cross(n, helper);
        let l1 = (p1[0] * p1[0] + p1[1] * p1[1] + p1[2] * p1[2]).sqrt();
        let p1 = [p1[0] / l1 * 0.12, p1[1] / l1 * 0.12, p1[2] / l1 * 0.12];
        let p2 = cross(n, p1);
        let l2 = (p2[0] * p2[0] + p2[1] * p2[1] + p2[2] * p2[2]).sqrt().max(1e-6);
        let p2 = [p2[0] / l2 * 0.12, p2[1] / l2 * 0.12, p2[2] / l2 * 0.12];
        let light = draw.light;
        draw.light = 0x0078_0078;
        for p in [p1, p2] {
            let c = |base: [f32; 3], s: f32| [base[0] + p[0] * s, base[1] + p[1] * s, base[2] + p[2] * s];
            let corners = [c(end, 1.0), c(start, 1.0), c(start, -1.0), c(end, -1.0)];
            game.native_draw_quad(draw, "powah:block/tile/beam", corners, [0.0, 0.0, 16.0, 16.0], TIER_COLORS[tier]);
            let back = [corners[3], corners[2], corners[1], corners[0]];
            game.native_draw_quad(draw, "powah:block/tile/beam", back, [0.0, 0.0, 16.0, 16.0], TIER_COLORS[tier]);
        }
        draw.light = light;
        return;
    }
    match machine(block) {
        Some(Mach::Furnator) if get_i(&entity.nbt, "burning", 0) != 0 => {
            // FurnatorRenderer: the lit face (block/furnator_lit) on its front, full bright, over the unlit one.
            let Some(f) = game.native_block_state_property(state, "facing").and_then(|f| FACE_NAMES.iter().position(|n| *n == f)) else { return };
            let e = 0.001;
            let corners = match f {
                2 => [[1.0, 1.0, -e], [1.0, 0.0, -e], [0.0, 0.0, -e], [0.0, 1.0, -e]],
                3 => [[0.0, 1.0, 1.0 + e], [0.0, 0.0, 1.0 + e], [1.0, 0.0, 1.0 + e], [1.0, 1.0, 1.0 + e]],
                4 => [[-e, 1.0, 0.0], [-e, 0.0, 0.0], [-e, 0.0, 1.0], [-e, 1.0, 1.0]],
                5 => [[1.0 + e, 1.0, 1.0], [1.0 + e, 0.0, 1.0], [1.0 + e, 0.0, 0.0], [1.0 + e, 1.0, 0.0]],
                _ => return,
            };
            let light = draw.light;
            draw.light = 0x00F0_00F0;
            game.native_draw_quad(draw, "powah:block/furnator_lit", corners, [0.0, 0.0, 16.0, 16.0], 0xFF_FFFF);
            draw.light = light;
        }
        Some(Mach::Magmator) => {
            // MagmatorRenderer: the fluid's surface in its basin, 0.51 up and up to 0.45 more as it fills, 0.625 wide.
            let (fluid, amount) = tank(entity);
            let Some(f) = fluid else { return };
            let y = 0.51 + amount as f32 * 0.45 / 4000.0;
            let (sprite, tint) = match f.as_str() {
                "minecraft:water" => ("block/water_still", 0x3F_76E4),
                "minecraft:lava" => ("block/lava_still", 0xFF_FFFF),
                _ => ("block/water_still", 0xFF_FFFF),
            };
            let (a, b) = (0.1875, 0.8125);
            game.native_draw_quad(draw, sprite, [[a, y, a], [a, y, b], [b, y, b], [b, y, a]], [0.0, 0.0, 16.0, 16.0], tint);
        }
        _ => {}
    }
}

/// `block_entity_render_bounding_box`: the built reactor stands 3x4x3 round its core; a beam reaches its orb.
pub(in crate::game) fn render_bounds(block: &str, entity: &ModBlockEntity, bounds: &mut [f32; 6]) {
    if is_reactor(block) && get_i(&entity.nbt, "hz_part", 0) == 0 {
        *bounds = [-1.0, 0.0, -1.0, 2.0, 4.0, 2.0];
    } else if is_rod(block) {
        let r = energizing_range() as f32 + 1.0;
        *bounds = [-r, -r, -r, r + 1.0, r + 1.0, r + 1.0];
    }
}

/// `block_entity_update_tag`: what each block's renderer reads.
pub fn update_tag(entity: &ModBlockEntity, tag: &mut Option<Compound>) {
    if !entity.id.starts_with("powah:") {
        return;
    }
    let mut out = Compound::new();
    for key in ["side_transfer_type", "hz_sides", "built", "hz_part", "running", "hz_fueled", "burning", "fluids", "OrbPos", "hz_cooldown", "hz_items", "hz_result"] {
        if let Some(v) = entity.nbt.get(&java(key)) {
            out.insert(java(key), v.clone());
        }
    }
    *tag = Some(out);
}

/// What a block's renderer shows, as a key: when it changes, clients are sent it again.
fn visual_key(entity: &ModBlockEntity) -> String {
    let mut k = String::new();
    for key in ["side_transfer_type", "hz_sides", "built", "running", "hz_fueled", "burning", "OrbPos", "hz_items", "hz_result"] {
        if let Some(v) = entity.nbt.get(&java(key)) {
            k.push_str(&format!("{key}={v:?};"));
        }
    }
    let (f, a) = tank(entity);
    k.push_str(&format!("{f:?}{}", a / 250));
    k.push_str(&format!("cool={}", get_i(&entity.nbt, "hz_cooldown", 0) >= 20));
    k
}

fn sync_visuals(game: &mut Game<'_>, entity: &mut ModBlockEntity, pos: Pos) {
    // The orb's items, as ids, for the client's floating items (slot 0 alone when a result waits).
    if entity.id == ORB {
        let result = entity.items.first().cloned().flatten();
        let ids: Vec<Tag> = match &result {
            Some(r) => vec![Tag::String(java(&r.item))],
            None => entity.items.iter().skip(1).flatten().map(|s| Tag::String(java(&s.item))).collect(),
        };
        entity.nbt.insert(java("hz_items"), Tag::List(ids.into_iter().collect()));
        set_i(&mut entity.nbt, "hz_result", i32::from(result.is_some()));
    }
    if machine(&entity.id) == Some(Mach::Reactor) {
        let fueled = get_i(&entity.nbt, "fuel", 0) > 0;
        set_i(&mut entity.nbt, "hz_fueled", i32::from(fueled));
    }
    let key = visual_key(entity);
    let old = match entity.nbt.get(&java("hz_vis")) {
        Some(Tag::String(s)) => s.to_string_lossy().to_string(),
        _ => String::new(),
    };
    if key != old {
        entity.nbt.insert(java("hz_vis"), Tag::String(java(&key)));
        game.native_block_entity_sync(pos);
    }
}

// ---- Placement (AbstractBlock.getStateForPlacement and facing())

/// Direction.getNearestLookingDirections: the six directions by how near each is to where the placer looks.
fn nearest_looking(yaw: f32, pitch: f32) -> [usize; 6] {
    let (y, p) = (yaw.to_radians(), pitch.to_radians());
    let look = [-y.sin() * p.cos(), -p.sin(), y.cos() * p.cos()];
    let dirs: [[f32; 3]; 6] = [[0.0, -1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0], [0.0, 0.0, 1.0], [-1.0, 0.0, 0.0], [1.0, 0.0, 0.0]];
    let mut order = [0, 1, 2, 3, 4, 5];
    let dot = |d: usize| dirs[d][0] * look[0] + dirs[d][1] * look[1] + dirs[d][2] * look[2];
    order.sort_by(|a, b| dot(*b).total_cmp(&dot(*a)));
    order
}

/// Whether the hopper may face `d` from `pos` (EnergyHopperBlock.isValidPosition): an inventory there, without energy.
fn hopper_faces(level: &super::GameLevel<'_>, pos: Pos, d: usize) -> bool {
    let there = step(pos, d);
    match level.world.block_entities.get(&(there[0], there[1], there[2])) {
        None => false,
        Some(horizon_client::containers::BlockEntity::Mod(m)) => !m.items.is_empty() && !m.nbt.contains_key(&java("energy")),
        Some(be) => be.slots().len() > 0,
    }
}

/// `state_for_placement`: the facing Powah's blocks take, as Lollipop places them.
pub(in crate::game) fn state_for_placement(level: &super::GameLevel<'_>, state: horizon_world::registry::BlockStateId, context: &horizon_world::level::PlacementContext, answer: &mut Option<Option<horizon_world::registry::BlockStateId>>) {
    let Some(block) = level.native_state_block(state).map(str::to_owned) else { return };
    let Some((kind, _)) = kind_tier(&block) else { return };
    let pos = [context.pos.x, context.pos.y, context.pos.z];
    let order = nearest_looking(context.yaw, context.pitch);
    let facing = match kind {
        // Facing.HORIZONTAL with isPlacerFacing: towards the placer.
        "furnator" | "magmator" | "energy_discharger" => {
            let y = context.yaw.rem_euclid(360.0);
            let looking = if !(45.0..315.0).contains(&y) { 3 } else if y < 135.0 { 4 } else if y < 225.0 { 2 } else { 5 };
            Some(opposite(looking))
        }
        // Facing.ALL: the nearest looking direction the block may face.
        "energy_hopper" => order.into_iter().find(|&d| hopper_faces(level, pos, d)),
        "ender_gate" | "energizing_rod" | "energizing_orb" => Some(order[0]),
        "reactor" => {
            *answer = Some(level.native_state_with_property(state, "core", "true"));
            return;
        }
        _ => return,
    };
    *answer = Some(facing.and_then(|d| level.native_state_with_property(state, "facing", FACE_NAMES[d])));
}

// ---- Breaking and placing: a cell keeps its energy and settings (keepEnergy, the storable data)

pub(in crate::game) fn block_drops(game: &mut Game<'_>, pos: Pos, block: &str, _explosion: bool, drops: &mut Vec<ItemStack>) {
    if is_reactor(block) {
        // A player's break was handled when it started (`destroyed`); an explosion's comes here.
        if STATE.with_borrow_mut(|s| s.demolished.remove(&pos)) {
            drops.retain(|d| &*d.item != block);
        } else {
            reactor_broken(game, pos, block, drops);
        }
        return;
    }
    if machine(block).is_none() || is_creative(block) {
        return;
    }
    let Some(horizon_client::containers::BlockEntity::Mod(e)) = game.world.block_entities.get(&(pos[0], pos[1], pos[2])) else { return };
    let stored = energy(e);
    for d in drops.iter_mut().filter(|d| &*d.item == block) {
        if let Some((_, mut filled)) = game.native_item_energy_insert(&ItemStack::new(block, 1), stored) {
            filled.count = d.count;
            *d = filled;
        }
    }
}

pub(in crate::game) fn placed(game: &mut Game<'_>, stack: &ItemStack, pos: Pos) {
    let block = game.native_block(pos);
    if matches!(machine(&block), Some(Mach::EnderCell | Mach::EnderGate)) {
        // AbstractEnderTile.onPlaced: its placer owns it.
        if let Some(player) = game.native_acting_player() {
            let ints: Vec<i32> = (0..4).map(|i| (player.uuid >> (96 - 32 * i)) as u32 as i32).collect();
            let name = player.name.clone();
            game.native_with_mod_block_entity(pos, |e| {
                if owner(e).is_none() {
                    e.nbt.insert(java("owner_id"), Tag::IntArray(ints.clone().into()));
                    e.nbt.insert(java("owner_name"), Tag::String(java(&name)));
                }
            });
        }
    }
    if is_reactor(&block) {
        reactor_placed(game, stack, pos);
        return;
    }
    if is_cable(&block) || kind_tier(&block).is_some() {
        forget_networks();
        // Cables around it join it.
        STATE.with_borrow_mut(|s| {
            for d in 0..6 {
                s.dirty.insert(step(pos, d));
            }
            s.dirty.insert(pos);
        });
    }
    if !is_cell(&block) || is_creative(&block) || &*stack.item != block {
        return;
    }
    let stored = game.native_item_energy(stack).map_or(0, |e| e.amount);
    if stored > 0 {
        game.native_with_mod_block_entity(pos, |e| set_energy(e, stored));
    }
}

// ---- Menu buttons (NextEnergyConfigPacket, NextRedstoneModePacket): checked and applied here, on the server

const B_SIDE: i32 = 0; // 0..5 one side, 6 all
const B_REDSTONE: i32 = 7;

pub fn menu_button(_pos: Pos, entity: &mut ModBlockEntity, id: i32) {
    if is_cable(&entity.id) {
        // NextEnergyConfigPacket on the menu's side; NextRedstoneModePacket.
        match id {
            0..=5 if id == get_i(&entity.nbt, "hz_menu_side", 2) => {
                let mut s = sides(entity);
                s[id as usize] = s[id as usize].next(Transfer::All);
                set_sides(entity, s);
            }
            B_REDSTONE => {
                let next = (redstone_mode(entity) + 1) % 3;
                set_i(&mut entity.nbt, "redstone_mode", next);
            }
            _ => {}
        }
        fill_cable_data(entity);
        return;
    }
    if machine(&entity.id).is_none() {
        return;
    }
    let allowed = transfer_type(&entity.id);
    let mut s = sides(entity);
    match id {
        0..=5 if energy_present(&entity.id, id as usize) => s[id as usize] = s[id as usize].next(allowed),
        0..=5 => return,
        6 => {
            // SideConfig.nextTypeAll: all one step on if they agree, otherwise all to ALL.
            if s[1..].iter().all(|t| *t == s[1]) {
                for t in &mut s {
                    *t = t.next(allowed);
                }
            } else {
                s = [Transfer::All; 6];
            }
            if allowed == Transfer::Extract {
                for t in &mut s {
                    if *t == Transfer::All || *t == Transfer::Receive {
                        *t = Transfer::Extract;
                    }
                }
            }
        }
        30 if machine(&entity.id) == Some(Mach::Reactor) => {
            // SwitchGenModePacket.
            let on = get_i(&entity.nbt, "gen_mode", 0) == 0;
            set_i(&mut entity.nbt, "gen_mode", i32::from(on));
            if !on {
                set_i(&mut entity.nbt, "generate", 1);
            }
        }
        10..=21 if matches!(machine(&entity.id), Some(Mach::EnderCell | Mach::EnderGate)) => {
            // SetChannelPacket: a channel the tier has.
            if id - 10 < max_channels(entity) {
                set_i(&mut entity.nbt, "channel", id - 10);
                set_i(&mut entity.nbt, "hz_shown", 0);
                set_i(&mut entity.nbt, "energy", 0);
            }
        }
        B_REDSTONE => {
            // Redstone.next: IGNORE, ON, OFF and round.
            let next = (redstone_mode(entity) + 1) % 3;
            set_i(&mut entity.nbt, "redstone_mode", next);
        }
        _ => return,
    }
    if id <= 6 {
        set_sides(entity, s);
    }
    fill_data(entity);
}

// ---- The screen (EnergyCellScreen on AbstractEnergyScreen and AbstractTileScreen)

pub fn screen(block: &str, out: &mut Option<super::native_hooks::NativeScreen>) {
    if is_cable(block) {
        // CableScreen: Textures.CABLE (153x29) and no inventory.
        *out = Some(super::native_hooks::NativeScreen { width: 153.0, height: 29.0, slots: vec![None], inventory_y: 10_000.0, inventory_x: 8.0, own_labels: true, own_background: true });
        return;
    }
    let Some(m) = machine(block) else { return };
    let (_, w, h, inv_y) = m.screen();
    *out = Some(super::native_hooks::NativeScreen {
        width: w,
        height: h,
        slots: m.slots().iter().map(|s| Some(*s)).collect(),
        inventory_y: inv_y,
        inventory_x: 8.0,
        own_labels: true,
        own_background: true,
    });
}

const BUTTON_OV: &str = "lollipop:gui/container/button_ov";

/// AbstractEnergyScreen.getSideButtonOffsets(6), by Direction index.
const SIDE_OFFSETS: [[f32; 2]; 6] = [[0.0, 6.0], [0.0, -6.0], [0.0, 0.0], [6.0, 6.0], [-6.0, 0.0], [6.0, 0.0]];

/// The side buttons' places from the screen's corner (x + xSize + 8 + offset, y + 4 + 10 + offset), 5x5 each; the
/// "all" button's; the redstone button's (x + xSize + 2, y + 31 + 3), 9x8.
fn side_rect(i: usize) -> [f32; 4] {
    [176.0 + 8.0 + SIDE_OFFSETS[i][0], 14.0 + SIDE_OFFSETS[i][1], 5.0, 5.0]
}
const ALL_RECT: [f32; 4] = [176.0 + 14.0, 8.0, 5.0, 5.0];
const REDSTONE_RECT: [f32; 4] = [176.0 + 2.0, 34.0, 9.0, 8.0];

fn inside(r: [f32; 4], at: [f32; 2]) -> bool {
    at[0] >= r[0] && at[0] < r[0] + r[2] && at[1] >= r[1] && at[1] < r[1] + r[3]
}

/// Texture.CONFIG: the side button's picture for its Transfer.
fn config_uv(t: Transfer) -> [f32; 2] {
    match t {
        Transfer::All => [28.0, 16.0],
        Transfer::Extract => [33.0, 16.0],
        Transfer::Receive => [38.0, 16.0],
        Transfer::None => [43.0, 16.0],
    }
}

/// Texture.REDSTONE: IGNORE, ON, OFF.
fn redstone_uv(mode: i32) -> [f32; 2] {
    [[38.0, 0.0], [38.0, 8.0], [47.0, 0.0]][mode.clamp(0, 2) as usize]
}

/// Util.numFormat: 1.2k, 4M, ...
fn num_format(v: i64) -> String {
    if v < 0 {
        return format!("-{}", num_format(-v));
    }
    if v < 1000 {
        return v.to_string();
    }
    let (div, suffix) = [(1_000_000_000_000_000_000i64, "E"), (1_000_000_000_000_000, "P"), (1_000_000_000_000, "T"), (1_000_000_000, "B"), (1_000_000, "M"), (1_000, "k")]
        .into_iter()
        .find(|(d, _)| v >= *d)
        .unwrap_or((1_000, "k"));
    let truncated = v / (div / 10);
    if truncated < 100 && truncated % 10 != 0 { format!("{}{}", truncated as f64 / 10.0, suffix) } else { format!("{}{}", truncated / 10, suffix) }
}

/// Util.addCommas: NumberFormat.getInstance (English grouping).
fn add_commas(v: i64) -> String {
    let s = v.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if v < 0 { format!("-{out}") } else { out }
}

fn lang(key: &str, english: &str) -> String {
    horizon_client::lang::get_or(key, english).to_string()
}

fn sprite(game: &Game<'_>, batch: &mut HudBatch, texture: &str, o: [f32; 2], at: [f32; 2], uv: [f32; 4]) {
    game.native_sprite(batch, texture, uv, [o[0] + at[0], o[1] + at[1], uv[2], uv[3]]);
}

/// A fluid's colour for its gauge: Horizon gives a mod screen no fluid textures, so the gauge is the fluid's tint.
fn fluid_color(kind: i32) -> u32 {
    // Water's biome tint, lava's colour, anything else grey; as Java writes them (0xAARRGGBB), turned for the batch.
    let argb: u32 = match kind {
        1 => 0xFF3F_76E4,
        2 => 0xFFD9_6415,
        _ => 0xFF88_88AA,
    };
    let [a, r, g, b] = argb.to_be_bytes();
    rgba(r, g, b, a)
}

/// CableScreen: the side's config button (CABLE_ALL/OUT/IN/OFF, 18x19 at 153/171/189/207,0) at 5,5; the redstone
/// button at xSize-21+2, 7+3 on its background; the side's name centred at 10 in 0x555555; the title above.
const CABLE_CONFIG_RECT: [f32; 4] = [5.0, 5.0, 18.0, 19.0];
const CABLE_REDSTONE_RECT: [f32; 4] = [153.0 - 21.0 + 2.0, 10.0, 9.0, 8.0];

fn cable_draw(game: &Game<'_>, block: &str, data: &[i32], batch: &mut HudBatch, origin: [f32; 2]) {
    let atlas = &game.gui.atlas;
    let tex = "powah:gui/container/cable";
    sprite(game, batch, tex, origin, [0.0, 0.0], [0.0, 0.0, 153.0, 29.0]);
    let side = data[D_CHANNEL].clamp(0, 5) as usize;
    let u = [153.0, 171.0, 189.0, 207.0][Transfer::from(data[D_SIDES + side]) as usize];
    sprite(game, batch, tex, origin, [CABLE_CONFIG_RECT[0], CABLE_CONFIG_RECT[1]], [u, 0.0, 18.0, 19.0]);
    sprite(game, batch, BUTTON_OV, origin, [CABLE_REDSTONE_RECT[0] - 2.0, CABLE_REDSTONE_RECT[1] - 4.0], [23.0, 0.0, 15.0, 16.0]);
    let uv = redstone_uv(data[D_REDSTONE]);
    sprite(game, batch, BUTTON_OV, origin, [CABLE_REDSTONE_RECT[0], CABLE_REDSTONE_RECT[1]], [uv[0], uv[1], 9.0, 8.0]);
    let title = lang(&format!("block.powah.{}", block.trim_start_matches("powah:")), block);
    let tw = HudBatch::text_width(atlas, &title);
    batch.text(atlas, &title, origin[0] + (76.0 - (tw / 2.0).floor()), origin[1] - 14.0, 0xFF99_9999, true);
    let name = lang(&format!("info.lollipop.side.{}", FACE_NAMES[side]), FACE_NAMES[side]).replacen("%s", "§8", 1);
    let nw = HudBatch::text_width(atlas, &name);
    batch.text(atlas, &name, origin[0] + (76.0 - (nw / 2.0).floor()), origin[1] + 10.0, 0xFF55_5555, false);
}

pub(in crate::game) fn screen_draw(game: &Game<'_>, block: &str, data: &[i32], batch: &mut HudBatch, origin: [f32; 2]) {
    if is_cable(block) && data.len() >= DATA_SLOTS {
        cable_draw(game, block, data, batch, origin);
        return;
    }
    let Some(m) = machine(block) else { return };
    if data.len() < DATA_SLOTS {
        return;
    }
    let atlas = &game.gui.atlas;
    let (tex, w, _, _) = m.screen();
    let (stored, cap) = (i64::from(wide(data, D_ENERGY)), i64::from(wide(data, D_CAPACITY)));
    let part = |v: i64, of: i64| if of > 0 { (v as f64 / of as f64).clamp(0.0, 1.0) as f32 } else { 0.0 };
    sprite(game, batch, tex, origin, [0.0, 0.0], [0.0, 0.0, w, m.screen().2]);
    match m {
        Mach::EnderCell | Mach::EnderGate => {
            // ENDER_CELL_GAUGE (139x37 at 0,164) at 31,6; the channel buttons (12x12, at 176,0 for the chosen one,
            // 188,0 the others) at 5 + 14i, 55; their numbers at half size.
            let fill = (139.0 * part(stored, cap)).floor();
            if fill > 0.0 {
                sprite(game, batch, tex, origin, [31.0, 6.0], [0.0, 164.0, fill, 37.0]);
            }
            let max = data[D_CHANNELS].clamp(0, 12);
            for i in 0..max {
                let u = if i == data[D_CHANNEL] { 176.0 } else { 188.0 };
                sprite(game, batch, tex, origin, [5.0 + 14.0 * i as f32, 55.0], [u, 0.0, 12.0, 12.0]);
            }
            let start = batch.vertices.len();
            for i in 1..13 {
                let f = if i > 9 { -2.0 } else { 0.0 };
                let x = 19.0 + 28.0 * (i - 1) as f32 + f;
                let color: u32 = if i <= max { 0xFF3E_8087 } else { 0x723E_8087 };
                let [a, r, g, b] = color.to_be_bytes();
                batch.text(atlas, &i.to_string(), origin[0] * 2.0 + x, origin[1] * 2.0 + 119.0, rgba(r, g, b, a), false);
            }
            batch.scale_since(start, 0.0, 0.0, 0.5, 0.5);
        }
        Mach::Cell | Mach::Transmitter => {
            // drawScalableW(subSized) at 31,6; the transmitter's light (6x6 at 176,0) at 9,5 with a card in.
            if m == Mach::Transmitter && data[D_BURNING] != 0 {
                sprite(game, batch, tex, origin, [9.0, 5.0], [176.0, 0.0, 6.0, 6.0]);
            }
            let fill = (139.0 * part(stored, cap)).floor();
            if fill > 0.0 {
                sprite(game, batch, tex, origin, [31.0, 6.0], [0.0, 141.0, fill, 37.0]);
            }
        }
        Mach::Solar | Mach::Discharger | Mach::Hopper => {
            // WIDE_ENERGY_GAUGE / DISCHARGER_GAUGE (164x37), drawScalableW at 6,6.
            let v = if m == Mach::Discharger { 166.0 } else { 141.0 };
            let fill = (164.0 * part(stored, cap)).floor();
            if fill > 0.0 {
                sprite(game, batch, tex, origin, [6.0, 6.0], [0.0, v, fill, 37.0]);
            }
        }
        Mach::Reactor => {
            // REACTOR_GAUGE (14x39 at 176,0) at 5,5; uraninite (5x48 at 190,16) at 103,13; carbon (5x16 at 190,0)
            // at 51,6; redstone (195,0) at 51,52; coolant (200,0) at 140,52; temperature (4x18 at 205,0) at 114,28;
            // all filled from the bottom (drawScalableH).
            let gauge = |batch: &mut HudBatch, v: f32, at: [f32; 2], uv: [f32; 4]| {
                let h = (uv[3] * v.clamp(0.0, 1.0)).floor();
                if h > 0.0 {
                    sprite(game, batch, tex, origin, [at[0], at[1] + uv[3] - h], [uv[0], uv[1] + uv[3] - h, uv[2], h]);
                }
            };
            let tk = |i: usize| part(i64::from(wide(data, D_R_FUEL + i * 4)), i64::from(wide(data, D_R_FUEL + i * 4 + 2)));
            gauge(batch, part(stored, cap), [5.0, 5.0], [176.0, 0.0, 14.0, 39.0]);
            gauge(batch, tk(0), [103.0, 13.0], [190.0, 16.0, 5.0, 48.0]);
            gauge(batch, tk(1), [51.0, 6.0], [190.0, 0.0, 5.0, 16.0]);
            gauge(batch, tk(2), [51.0, 52.0], [195.0, 0.0, 5.0, 16.0]);
            gauge(batch, tk(3), [140.0, 52.0], [200.0, 0.0, 5.0, 16.0]);
            gauge(batch, tk(4), [114.0, 28.0], [205.0, 0.0, 4.0, 18.0]);
            if data[D_FLUID] != 0 {
                let fh = (65.0 * part(i64::from(data[D_FLUID_AMOUNT]), 1000)).floor();
                if fh > 0.0 {
                    batch.fill(atlas, origin[0] + 157.0, origin[1] + 5.0 + 65.0 - fh, 14.0, fh, fluid_color(data[D_FLUID]));
                }
            }
            // The generation mode button (8x8 at 224,0 off / 224,8 on) at -11,10, on REACTOR_GEN_MODE_BG at -15,6.
            sprite(game, batch, tex, origin, [-15.0, 6.0], [209.0, 0.0, 15.0, 16.0]);
            let v = if data[D_R_GEN_MODE] != 0 { 8.0 } else { 0.0 };
            sprite(game, batch, tex, origin, [-11.0, 10.0], [224.0, v, 8.0, 8.0]);
        }
        Mach::Furnator | Mach::Magmator | Mach::Thermo => {
            // The energy gauge (14x39 at 176,0), filled from the bottom, at 5,5.
            let h = (39.0 * part(stored, cap)).floor();
            if h > 0.0 {
                sprite(game, batch, tex, origin, [5.0, 5.0 + 39.0 - h], [176.0, 39.0 - h, 14.0, h]);
            }
            if m == Mach::Furnator {
                // The carbon gauge (4x16 at 190,0) at 110,18, and the fire (11x18 at 176,39) at 94,43 while burning.
                let ch = (16.0 * part(i64::from(wide(data, D_FUEL)), i64::from(wide(data, D_FUEL_MAX)))).floor();
                if ch > 0.0 {
                    sprite(game, batch, tex, origin, [110.0, 18.0 + 16.0 - ch], [190.0, 16.0 - ch, 4.0, ch]);
                }
                if data[D_BURNING] != 0 {
                    sprite(game, batch, tex, origin, [94.0, 43.0], [176.0, 39.0, 11.0, 18.0]);
                }
            }
            if m == Mach::Magmator && data[D_BURNING] != 0 {
                sprite(game, batch, tex, origin, [83.0, 29.0], [176.0, 39.0, 11.0, 18.0]);
            }
            if matches!(m, Mach::Magmator | Mach::Thermo | Mach::Reactor) && data[D_FLUID] != 0 {
                // Draw.gaugeV: the tank (14x65 at 157,5): 4,000 mB, the reactor's 1,000.
                let of = if m == Mach::Reactor { 1000 } else { 4000 };
                let fh = (65.0 * part(i64::from(data[D_FLUID_AMOUNT]), of)).floor();
                if fh > 0.0 {
                    batch.fill(atlas, origin[0] + 157.0, origin[1] + 5.0 + 65.0 - fh, 14.0, fh, fluid_color(data[D_FLUID]));
                }
            }
            if m == Mach::Thermo {
                // "percent% (n EF/t)" at 34,10, 0x555555.
                let per_tick = generation_of(block);
                let now = wide(data, D_GENERATION);
                let percent = if per_tick > 0 { 100 * i64::from(now) / i64::from(per_tick) } else { 0 };
                let line = format!("{percent}% ({now} EF/t)");
                batch.text(atlas, &line, origin[0] + 34.0, origin[1] + 10.0, 0xFF55_5555, false);
            }
        }
    }
    // AbstractTileScreen: the redstone button on REDSTONE_BTN_BG; AbstractEnergyScreen: the side buttons on CONFIG_BTN_BG.
    sprite(game, batch, BUTTON_OV, origin, [REDSTONE_RECT[0] - 2.0, REDSTONE_RECT[1] - 4.0], [23.0, 0.0, 15.0, 16.0]);
    let uv = redstone_uv(data[D_REDSTONE]);
    sprite(game, batch, BUTTON_OV, origin, [REDSTONE_RECT[0], REDSTONE_RECT[1]], [uv[0], uv[1], 9.0, 8.0]);
    let up = side_rect(1);
    sprite(game, batch, BUTTON_OV, origin, [up[0] - 8.0, up[1] - 4.0], [0.0, 0.0, 23.0, 25.0]);
    for i in 0..6 {
        let r = side_rect(i);
        let uv = config_uv(Transfer::from(data[D_SIDES + i]));
        sprite(game, batch, BUTTON_OV, origin, [r[0], r[1]], [uv[0], uv[1], 5.0, 5.0]);
    }
    sprite(game, batch, BUTTON_OV, origin, [ALL_RECT[0], ALL_RECT[1]], [23.0, 16.0, 5.0, 5.0]);
    // drawForeground: the title 14 above, centred, 0x999999 with a shadow.
    let title = lang(&format!("block.powah.{}", block.trim_start_matches("powah:")), block);
    let tw = HudBatch::text_width(atlas, &title);
    batch.text(atlas, &title, origin[0] + (w / 2.0 - (tw / 2.0).floor()), origin[1] - 14.0, 0xFF99_9999, true);
    if matches!(m, Mach::Cell | Mach::Solar | Mach::Discharger | Mach::Hopper | Mach::Transmitter | Mach::EnderCell | Mach::EnderGate) {
        // EnergyCellScreen at 38, SolarScreen and DischargerScreen at 12: the energy and transfer lines (0x66000000).
        let lx = if matches!(m, Mach::Cell | Mach::Transmitter | Mach::EnderCell | Mach::EnderGate) { 38.0 } else { 12.0 };
        let creative = data[D_CREATIVE] != 0;
        let energy_line = if creative { lang("info.powah.unlimited", "Unlimited") } else { format!("{}/{} FE", add_commas(stored), num_format(cap)) };
        // The ender cell writes its lines in 0x724AFFDE.
        let line_color = if matches!(m, Mach::EnderCell | Mach::EnderGate) {
            let [a, r, g, b] = 0x724A_FFDEu32.to_be_bytes();
            rgba(r, g, b, a)
        } else {
            0x6600_0000
        };
        batch.text(atlas, &energy_line, origin[0] + lx, origin[1] + 13.0, line_color, false);
        let transfer_line = format!("{} FE/t", num_format(i64::from(wide(data, D_TRANSFER))));
        batch.text(atlas, &transfer_line, origin[0] + lx, origin[1] + 27.0, line_color, false);
    }
}

pub fn screen_click(block: &str, data: &[i32], at: [f32; 2], _right: bool, _shift: bool, button: &mut Option<i32>) {
    if is_cable(block) && data.len() >= DATA_SLOTS {
        if inside(CABLE_CONFIG_RECT, at) {
            *button = Some(data[D_CHANNEL].clamp(0, 5));
        } else if inside(CABLE_REDSTONE_RECT, at) {
            *button = Some(B_REDSTONE);
        }
        return;
    }
    if machine(block).is_none() || data.len() < DATA_SLOTS {
        return;
    }
    if matches!(machine(block), Some(Mach::EnderCell | Mach::EnderGate)) {
        if let Some(i) = (0..data[D_CHANNELS].clamp(0, 12)).find(|&i| inside([5.0 + 14.0 * i as f32, 55.0, 12.0, 12.0], at)) {
            *button = Some(10 + i);
            return;
        }
    }
    if machine(block) == Some(Mach::Reactor) && inside([-11.0, 10.0, 8.0, 8.0], at) {
        *button = Some(30);
        return;
    }
    if inside(REDSTONE_RECT, at) {
        *button = Some(B_REDSTONE);
    } else if inside(ALL_RECT, at) {
        *button = Some(6);
    } else if let Some(i) = (0..6).find(|&i| inside(side_rect(i), at)) {
        *button = Some(B_SIDE + i as i32);
    }
}

pub fn screen_tooltip(block: &str, data: &[i32], at: [f32; 2], lines: &mut Vec<String>) {
    if is_cable(block) && data.len() >= DATA_SLOTS {
        let side = data[D_CHANNEL].clamp(0, 5) as usize;
        if inside(CABLE_CONFIG_RECT, at) {
            // The side, then Transfer.getDisplayName2: push / pull for extract / receive.
            lines.push(format!("§7{}", lang(&format!("info.lollipop.side.{}", FACE_NAMES[side]), FACE_NAMES[side]).replacen("%s", "§8", 1)));
            let t = Transfer::from(data[D_SIDES + side]);
            let key = match t {
                Transfer::Extract => "push",
                Transfer::Receive => "pull",
                other => other.key(),
            };
            let color = if t == Transfer::None { "§4" } else { "§8" };
            lines.push(format!("§7{}: {color}{}", lang("info.lollipop.io.mode", "Mode"), lang(&format!("info.lollipop.io.mode.{key}"), key)));
        } else if inside(CABLE_REDSTONE_RECT, at) {
            let (key, color) = [("ignore", "§8"), ("on", "§c"), ("off", "§4")][data[D_REDSTONE].clamp(0, 2) as usize];
            lines.push(format!("§7{}: {color}{}", lang("info.lollipop.redstone", "Redstone"), lang(&format!("info.lollipop.{key}"), key)));
        }
        return;
    }
    let Some(m) = machine(block) else { return };
    if data.len() < DATA_SLOTS {
        return;
    }
    let colon = ": ";
    let mode_line = |t: Transfer| {
        let color = if t == Transfer::None { "§4" } else { "§8" };
        format!("§7{}{colon}{color}{}", lang("info.lollipop.io.mode", "Mode"), lang(&format!("info.lollipop.io.mode.{}", t.key()), t.key()))
    };
    if inside(REDSTONE_RECT, at) {
        let (key, color) = [("ignore", "§8"), ("on", "§c"), ("off", "§4")][data[D_REDSTONE].clamp(0, 2) as usize];
        lines.push(format!("§7{}{colon}{color}{}", lang("info.lollipop.redstone", "Redstone"), lang(&format!("info.lollipop.{key}"), key)));
    } else if inside(ALL_RECT, at) {
        lines.push(format!("§7{}{colon}§8{}", lang("info.lollipop.facing", "Facing"), lang("info.lollipop.all", "All")));
        lines.push(mode_line(Transfer::from(data[D_SIDES + 1])));
    } else if let Some(i) = (0..6).find(|&i| inside(side_rect(i), at)) {
        lines.push(format!("§7{}{colon}§8{}", lang("info.lollipop.facing", "Facing"), lang(&format!("info.lollipop.side.{}", FACE_NAMES[i]), FACE_NAMES[i])));
        lines.push(mode_line(Transfer::from(data[D_SIDES + i])));
    } else if m.generator() && m != Mach::Solar && inside([5.0, 5.0, 14.0, 39.0], at) {
        // The energy gauge: stored, generation, max extract.
        let (stored, cap) = (i64::from(wide(data, D_ENERGY)), i64::from(wide(data, D_CAPACITY)));
        let per_tick = lang("info.lollipop.fe.pet.tick", " FE/t");
        lines.push(format!("§7{}{colon}§8{}", lang("info.lollipop.stored", "Stored"), lang("info.lollipop.fe.stored", "%s / %s FE").replacen("%s", &add_commas(stored), 1).replacen("%s", &num_format(cap), 1)));
        lines.push(format!("§7{}{colon}§8{}{per_tick}", lang("info.lollipop.generates", "Generates"), num_format(i64::from(generation_of(block)))));
        lines.push(format!("§7{}{colon}§8{}{per_tick}", lang("info.lollipop.max.extract", "Max Extract"), num_format(i64::from(wide(data, D_TRANSFER)))));
    } else if m == Mach::Furnator && inside([110.0, 18.0, 4.0, 16.0], at) {
        lines.push(format!("§8{}", lang("info.powah.carbon", "Carbon")));
    } else if matches!(m, Mach::Magmator | Mach::Thermo) && inside([157.0, 5.0, 14.0, 65.0], at) {
        if data[D_FLUID] != 0 {
            let name = match data[D_FLUID] {
                1 => lang("block.minecraft.water", "Water"),
                2 => lang("block.minecraft.lava", "Lava"),
                _ => lang("info.lollipop.fluid", "Fluid"),
            };
            lines.push(format!("§7{}", name));
            lines.push(format!("§7{}{colon}§8{}", lang("info.lollipop.stored", "Stored"), lang("info.lollipop.mb.stored", "%s / %s mB").replacen("%s", &add_commas(i64::from(data[D_FLUID_AMOUNT])), 1).replacen("%s", "4k", 1)));
        } else {
            lines.push(format!("§7{}", lang("info.lollipop.empty", "Empty")));
        }
    }
}



pub(in crate::game) fn container_accepts(game: &Game<'_>, entity: &ModBlockEntity, _slot: usize, stack: &ItemStack, _face: Option<Direction>, cancel: &mut bool) {
    if entity.id == ORB {
        // EnergizingOrbTile.canInsert: an ingredient slot, empty, while no result waits; one each (getSlotLimit).
        if _slot == 0 || entity.items.first().cloned().flatten().is_some() || entity.items.get(_slot).cloned().flatten().is_some() {
            *cancel = true;
        }
        let _ = (game, stack);
        return;
    }
    match machine(&entity.id) {
        // canInsert: the furnator's slot 1 takes fuel, every other slot what holds energy.
        Some(Mach::Furnator) if _slot == 1 => {
            if game.res.providers().burn_time(game.res.items(), stack, false) == 0 {
                *cancel = true;
            }
        }
        Some(Mach::Hopper) => *cancel = true,
        Some(Mach::Reactor) => {
            // ReactorTile.canInsert.
            let ok = match _slot {
                1 => &*stack.item == "powah:uraninite",
                2 => game.res.providers().burn_time(game.res.items(), stack, false) > 0 && game.res.items().get(&stack.item).and_then(|i| i.crafting_remainder.as_deref()).is_none(),
                3 => matches!(&*stack.item, "minecraft:redstone" | "minecraft:redstone_block"),
                4 => solid_coolant(&stack.item).is_some(),
                _ => game.native_item_energy(stack).is_some(),
            };
            if !ok || entity.data.get(D_R_PART).is_some_and(|p| *p != 0) {
                *cancel = true;
            }
        }
        Some(m @ (Mach::EnderCell | Mach::EnderGate)) => {
            let ok = if _slot == 0 { m == Mach::EnderCell && ((is_cell(&stack.item) && !is_creative(&stack.item)) || is_battery(&stack.item)) } else { game.native_item_energy(stack).is_some() };
            if !ok {
                *cancel = true;
            }
        }
        Some(Mach::Transmitter) => {
            if bound(stack).is_none() {
                *cancel = true;
            }
        }
        Some(Mach::Discharger) => {
            if !game.native_item_energy(stack).is_some_and(|e| e.amount > 0 && e.max_extract > 0) {
                *cancel = true;
            }
        }
        Some(_) if game.native_item_energy(stack).is_none() => *cancel = true,
        _ => {}
    }
}

pub(in crate::game) fn kit_frame(game: &mut Game<'_>) {
    if !game.native_on(NS) {
        return;
    }
    kit_guide::register(NS, "Powah", "Powah guide");
    powah_optional::if_mod!("jei", {
        let mut p = kit_layer::Provider::new(NS);
        p.is_catalyst = |item| {
            item == ORB
                || item.strip_prefix("powah:").is_some_and(|n| ["energizing_rod_", "magmator_", "thermo_generator_", "reactor_"].iter().any(|k| n.starts_with(k)))
        };
        p.tabs = jei_tabs;
        kit_layer::register(p);
    } else { () });
}

// ---- JEI (PowahJEIPlugin and its categories: Energizing, Magmatic Fluids, Coolants, Solid Coolants, Heat Sources)
powah_optional::if_mod_items!("jei", {
use crate::game::jei::Stack;

/// Lollipop's text colour in Powah's categories (4473924).
const JEI_TEXT: u32 = 0xFF44_4444;

fn jei_text(at: [f32; 2], text: String, color: u32) -> kit_window::PictureText {
    kit_window::PictureText { at, text, color, hover: color, right: false, tip: Vec::new() }
}

/// A card on one of Powah's JEI backgrounds (160 wide). Places are on the texture: JEI's `addPadding(1, 0, 0, 0)` moves
/// the background and everything on it down together, so an item set at (x, y) in the category lies at (x+1, y) on
/// the texture (inside its slot frame, 4,4 on both of Powah's), and a line drawn at y lies at y-1.
fn jei_card(sprite: &str, h: f32, slots: Vec<([f32; 2], Vec<Stack>)>, texts: Vec<kit_window::PictureText>, fluids: Vec<kit_window::PictureFluid>) -> kit_window::Card {
    kit_window::Card::Picture {
        sprite: sprite.to_owned(),
        src: [0.0, 0.0, 160.0, h],
        covers: Vec::new(),
        slots: slots.into_iter().map(|(at, s)| (at, s, Vec::new())).collect(),
        items: Vec::new(),
        texts,
        hovers: Vec::new(),
        fluids,
    }
}

/// A fluid slot (JEI's 1,000 mB): the fluid's still texture only. The categories also set the bucket as an item
/// ingredient, for lookups only (`setInput(VanillaTypes.ITEM, ...)`); JEI draws the fluid stacks the layout inits.
fn jei_fluid(fluid: &str) -> kit_window::PictureFluid {
    kit_window::PictureFluid { at: [4.0, 4.0], fluid: fluid.to_owned(), amount: 1000, ..Default::default() }
}

/// A fluid as JEI's list shows it here: its bucket (the kit's stacks are items).
fn bucket(fluid: &str) -> Stack {
    Stack::item(match fluid {
        "minecraft:water" => "minecraft:water_bucket",
        "minecraft:lava" => "minecraft:lava_bucket",
        other => other,
    })
}

fn ingredient(game: &Game<'_>, json: &serde_json::Value) -> Vec<Stack> {
    let j = match json {
        serde_json::Value::String(s) => match s.strip_prefix('#') {
            Some(tag) => serde_json::json!({ "tag": tag }),
            None => serde_json::json!({ "item": s }),
        },
        other => other.clone(),
    };
    match game.native_recipe_ingredient(&j) {
        Some(i) => i.order.iter().map(|id| Stack::item(id)).collect(),
        None => j["item"].as_str().map(|id| vec![Stack::item(id)]).unwrap_or_default(),
    }
}

fn jei_tabs(game: &Game<'_>, item: &Stack, uses: bool) -> Vec<kit_window::Tab> {
    let id = item.item.as_str();
    let mut tabs = Vec::new();
    let tiers = |kind: &str| TIERS.iter().map(|t| format!("powah:{kind}_{t}")).collect::<Vec<_>>();
    let catalyst = |kind: &str| id.strip_prefix("powah:").is_some_and(|n| n.starts_with(kind));
    // Energizing (EnergizingCategory: 160x38, ingredients at 20i+3,4, the result at 136,4, "%s FE" at 2,29).
    let all = catalyst("energizing_") || id == ORB;
    let mut cards = Vec::new();
    for r in game.native_recipes_of_type("powah:energizing").iter() {
        let list: Vec<Vec<Stack>> = r.json["ingredients"].as_array().map(|a| a.iter().map(|i| ingredient(game, i)).collect()).unwrap_or_default();
        let result = &r.json["result"];
        let out = Stack { item: result["id"].as_str().or_else(|| result["item"].as_str()).unwrap_or("minecraft:air").to_owned(), count: result["count"].as_u64().unwrap_or(1) as u32, components: String::new() };
        let shows = all || if uses { list.iter().any(|s| s.iter().any(|x| x.item == id)) } else { out.item == id };
        if !shows {
            continue;
        }
        let mut slots: Vec<([f32; 2], Vec<Stack>)> = list.into_iter().enumerate().map(|(i, s)| ([i as f32 * 20.0 + 4.0, 4.0], s)).collect();
        slots.push(([137.0, 4.0], vec![out]));
        let fe = lang("info.lollipop.fe", "%s FE").replacen("%s", &add_commas(r.json["energy"].as_i64().unwrap_or(0)), 1);
        cards.push(jei_card("powah:gui/jei/energizing", 38.0, slots, vec![jei_text([2.0, 28.0], fe, JEI_TEXT)], Vec::new()));
    }
    if !cards.is_empty() {
        let mut catalysts = vec![ORB.to_owned()];
        catalysts.extend(tiers("energizing_rod"));
        tabs.push(kit_window::Tab { icon: ORB.to_owned(), title: lang("gui.powah.jei.category.energizing", "Energizing"), catalysts, cards });
    }
    // Magmatic Fluids (MagmatorCategory: the fluid at 4,5, "heat FE/100 mb" at 27,9).
    let fluids = list_config("generators/magmator", "magmaticFluids", &["minecraft:lava=10000"]);
    let all = catalyst("magmator_");
    let cards: Vec<kit_window::Card> = fluids
        .iter()
        .filter(|(f, _)| all || bucket(f).item == id || *f == id)
        .map(|(f, heat)| jei_card("powah:gui/jei/misc", 24.0, Vec::new(), vec![jei_text([27.0, 8.0], format!("{heat} FE/100 mb"), JEI_TEXT)], vec![jei_fluid(f)]))
        .collect();
    if !cards.is_empty() {
        tabs.push(kit_window::Tab { icon: "powah:magmator_basic".to_owned(), title: lang("gui.powah.jei.category.magmatic", "Magmatic Fluids"), catalysts: tiers("magmator"), cards });
    }
    let temp = |v: String| format!("{}: {}", lang("info.lollipop.temperature", "Temp"), lang("info.lollipop.temperature.c", "%s C").replacen("%s", &v, 1));
    // Coolants (CoolantCategory: the fluid at 4,5, the temperature at 30,9).
    let all = catalyst("thermo_generator_") || catalyst("reactor_");
    let cards: Vec<kit_window::Card> = list_config("general_common", "coolantFluids", &["minecraft:water=1"])
        .iter()
        .filter(|(f, _)| all || bucket(f).item == id || *f == id)
        .map(|(f, cold)| jei_card("powah:gui/jei/misc", 24.0, Vec::new(), vec![jei_text([30.0, 8.0], temp(format!("§3{cold}")), JEI_TEXT)], vec![jei_fluid(f)]))
        .collect();
    if !cards.is_empty() {
        let mut catalysts = tiers("thermo_generator");
        catalysts.extend(tiers("reactor"));
        tabs.push(kit_window::Tab { icon: "minecraft:water_bucket".to_owned(), title: lang("gui.powah.jei.category.coolant", "Coolants"), catalysts, cards });
    }
    // Solid Coolants (SolidCoolantCategory: the item at 3,4, the amount at 30,3, the temperature at 30,15).
    let all = catalyst("reactor_");
    let solids = ["minecraft:snow_block", "minecraft:snowball", "minecraft:ice", "minecraft:packed_ice", "minecraft:blue_ice", "powah:dry_ice"];
    let cards: Vec<kit_window::Card> = solids
        .iter()
        .filter(|s| all || **s == id)
        .filter_map(|s| solid_coolant(s).map(|c| (s, c)))
        .map(|(s, (amount, cold))| {
            let a = format!("{}: {}", lang("info.lollipop.amount", "Amount"), lang("info.lollipop.mb", "%s mb").replacen("%s", &amount.to_string(), 1));
            jei_card("powah:gui/jei/misc", 24.0, vec![([4.0, 4.0], vec![Stack::item(s)])], vec![jei_text([30.0, 2.0], a, JEI_TEXT), jei_text([30.0, 14.0], temp(format!("§3{cold}")), JEI_TEXT)], Vec::new())
        })
        .collect();
    if !cards.is_empty() {
        tabs.push(kit_window::Tab { icon: "powah:dry_ice".to_owned(), title: lang("gui.powah.jei.category.solid.coolant", "Solid Coolants"), catalysts: tiers("reactor"), cards });
    }
    // Heat Sources (HeatSourceCategory: a block at 3,4 or a fluid at 4,5, "Temp: heat C" at 30,9 in 0xC43400).
    let all = catalyst("thermo_generator_");
    let sources = list_config("general_common", "heatSources", &["minecraft:lava=1000", "minecraft:magma_block=800", "powah:blazing_crystal_block=2800"]);
    let cards: Vec<kit_window::Card> = sources
        .iter()
        .filter(|(b, _)| all || **b == id || bucket(b).item == id)
        .map(|(b, heat)| {
            let line = format!("§8{}: §r{}", lang("info.lollipop.temperature", "Temp"), lang("info.lollipop.temperature.c", "%s C").replacen("%s", &heat.to_string(), 1));
            // 12858368 (0xC43400), as Java's ARGB: picture text takes it so.
            let color = 0xFFC4_3400u32;
            let fluid = matches!(b.as_str(), "minecraft:lava" | "minecraft:water");
            let (slots, fluids) = if fluid { (Vec::new(), vec![jei_fluid(b)]) } else { (vec![([4.0, 4.0], vec![Stack::item(b)])], Vec::new()) };
            jei_card("powah:gui/jei/misc", 24.0, slots, vec![jei_text([30.0, 8.0], line, color)], fluids)
        })
        .collect();
    if !cards.is_empty() {
        tabs.push(kit_window::Tab { icon: "minecraft:magma_block".to_owned(), title: lang("gui.powah.jei.category.heat.sources", "Heat Sources"), catalysts: tiers("thermo_generator"), cards });
    }
    // addIngredientInfo: the player aerial pearl, the dimensional binding card, the lens of ender.
    for (item, key) in [("powah:player_aerial_pearl", "jei.powah.player_aerial_pearl"), ("powah:binding_card_dim", "jei.powah.binding_card_dim"), ("powah:lens_of_ender", "jei.powah.lens_of_ender")] {
        if id == item {
            let text = lang(key, "");
            // A lang line breaks at a written "\n" as well as a real one.
            let lines: Vec<String> = text.replace("\\n", "\n").lines().map(str::to_owned).collect();
            tabs.push(kit_window::Tab { icon: item.to_owned(), title: lang("jei.information", "Information"), catalysts: Vec::new(), cards: vec![kit_window::Card::Text(lines)] });
        }
    }
    tabs
}
});



/// `interact_entity` (BindingCardItem / PhotoelectricPaneItem.itemInteractionForEntity, on the server): a binding card
/// used on an enderman or an endermite (the classes themselves, not a subclass) becomes a dimensional card, keeping
/// whom it is bound to; a photoelectric pane becomes a lens of ender (one used, unless creative, the lens given or
/// dropped). The mob is taken away with the enderman's death sound at it, as Java's `remove()` does: no loot.
/// Both are on by default in Powah's general config (`binding_card_dim`, `lens_of_ender`).
pub(in crate::game) fn interact_entity(game: &mut Game<'_>, entity: i32, kind: &str, stack: Option<&ItemStack>, claimed: &mut bool) {
    if *claimed || !game.native_on(NS) || !game.native_authoritative() {
        return;
    }
    if kind != "minecraft:enderman" && kind != "minecraft:endermite" {
        return;
    }
    let Some(item) = stack.map(|s| s.item.to_string()) else { return };
    if item != CARD && item != "powah:photoelectric_pane" {
        return;
    }
    let Some(player) = game.native_acting_player() else { return };
    let slot = game.selected as i32;
    let Some(at) = game.mobs.iter().find(|m| m.id as i32 == entity && !m.removed).map(|m| [m.position.x, m.position.y, m.position.z]) else { return };
    let creative = game.creative;
    if item == CARD {
        let Some(held) = game.native_carried_stack(slot).filter(|h| &*h.item == CARD) else { return };
        let mut card = ItemStack::new(CARD_DIM, 1);
        if let Some(c) = custom(held) {
            let mut kept = Compound::default();
            for key in ["bound_player_id", "bound_player_name"] {
                if let Some(v) = c.get(&java(key)) {
                    kept.insert(java(key), v.clone());
                }
            }
            if !kept.is_empty() {
                card.extra.get_or_insert_with(Default::default).other.push(("minecraft:custom_data".into(), Tag::Compound(kept)));
            }
        }
        *held = card;
    } else {
        let Some(held) = game.native_carried_stack(slot).filter(|h| &*h.item == "powah:photoelectric_pane") else { return };
        if !creative {
            held.count -= 1;
            if held.count <= 0 {
                *held = ItemStack::new("minecraft:air", 0);
            }
        }
        if let Some(rest) = game.native_give_stack(player.uuid, ItemStack::new("powah:lens_of_ender", 1)) {
            game.spawn_item(rest, at, [0.0, 0.0, 0.0], 10);
        }
    }
    let _ = game.native_play_sound_in_place("minecraft:entity.enderman.death", at, 5, 0.5, 1.0);
    let dim = game.native_dimension().to_owned();
    game.native_remove_mob(&dim, entity as u32);
    *claimed = true;
}
