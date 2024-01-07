use anyhow::{anyhow, Context};
use derive_more::{Deref, DerefMut};
use indexmap::IndexSet;
use itertools::Itertools;

use crate::protogen;

#[derive(Debug, Clone, Deref, DerefMut)]
pub(crate) struct Types(pub(crate) IndexSet<Type>);

#[derive(Debug, Clone, Deref, DerefMut)]
pub(crate) struct ModifiedTypes(pub(crate) IndexSet<Type>);

#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut)]
pub(crate) struct AddTypes(pub(crate) IndexSet<Type>);

#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut)]
pub(crate) struct RemoveTypes(pub(crate) IndexSet<Type>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RemoveAllTypes;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, strum::AsRefStr, strum::EnumString,
)]
pub enum Type {
    Legendary,
    World,
    Tribal,
    Instant,
    Sorcery,
    Creature,
    Artifact,
    Enchantment,
    Battle,
    Snow,
    Land,
    Planeswalker,
    Stickers,
    Basic,
}

impl TryFrom<&protogen::types::Type> for Type {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::types::Type) -> Result<Self, Self::Error> {
        value
            .type_
            .as_ref()
            .ok_or_else(|| anyhow!("Expected type to have a type set"))
            .map(Self::from)
    }
}

impl From<&protogen::types::type_::Type> for Type {
    fn from(value: &protogen::types::type_::Type) -> Self {
        match value {
            protogen::types::type_::Type::Basic(_) => Self::Basic,
            protogen::types::type_::Type::Land(_) => Self::Land,
            protogen::types::type_::Type::Instant(_) => Self::Instant,
            protogen::types::type_::Type::Sorcery(_) => Self::Sorcery,
            protogen::types::type_::Type::Creature(_) => Self::Creature,
            protogen::types::type_::Type::Artifact(_) => Self::Artifact,
            protogen::types::type_::Type::Enchantment(_) => Self::Enchantment,
            protogen::types::type_::Type::Battle(_) => Self::Battle,
            protogen::types::type_::Type::Legendary(_) => Self::Legendary,
            protogen::types::type_::Type::Planeswalker(_) => Self::Planeswalker,
            protogen::types::type_::Type::Snow(_) => Self::Snow,
            protogen::types::type_::Type::Stickers(_) => Self::Stickers,
            protogen::types::type_::Type::Tribal(_) => Self::Tribal,
            protogen::types::type_::Type::World(_) => Self::World,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut)]
pub(crate) struct Subtypes(pub(crate) IndexSet<Subtype>);

#[derive(Debug, Clone, Deref, DerefMut)]
pub(crate) struct ModifiedSubtypes(pub(crate) IndexSet<Subtype>);

#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut)]
pub(crate) struct AddSubtypes(pub(crate) IndexSet<Subtype>);

#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut)]
pub(crate) struct RemoveSubtypes(pub(crate) IndexSet<Subtype>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RemoveAllCreatureTypes;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, strum::AsRefStr, strum::EnumString,
)]
pub enum Subtype {
    Adventure,
    Advisor,
    Aetherborn,
    Ajani,
    Alien,
    Ally,
    Aminatou,
    Angel,
    Angrath,
    Antelope,
    Ape,
    Arcane,
    Archer,
    Archon,
    Arlinn,
    Army,
    Artificer,
    Ashiok,
    Assassin,
    AssemblyWorker,
    Astartes,
    Atog,
    Attraction,
    Aura,
    Aurochs,
    Avatar,
    Azra,
    Background,
    Badger,
    Bahamut,
    Balloon,
    Barbarian,
    Bard,
    Basilisk,
    Basri,
    Bat,
    Bear,
    Beast,
    Beeble,
    Beholder,
    Berserker,
    Bird,
    Blinkmoth,
    Boar,
    Bolas,
    Bringer,
    Brushwagg,
    Calix,
    Camarid,
    Camel,
    Capybara,
    Caribou,
    Carrier,
    Cartouche,
    Cat,
    Cave,
    Centaur,
    Cephalid,
    Chandra,
    Child,
    Chimera,
    Citizen,
    Class,
    Cleric,
    Clown,
    Clue,
    Cockatrice,
    Comet,
    Construct,
    Coward,
    Crab,
    Crocodile,
    Ctan,
    Curse,
    Custodes,
    Cyberman,
    Cyclops,
    Dack,
    Dakkon,
    Dalek,
    Daretti,
    Dauthi,
    Davriel,
    Demigod,
    Demon,
    Desert,
    Deserter,
    Detective,
    Devil,
    Dihada,
    Dinosaur,
    Djinn,
    Doctor,
    Dog,
    Domri,
    Dovin,
    Dragon,
    Drake,
    Dreadnought,
    Drone,
    Druid,
    Dryad,
    Dwarf,
    Efreet,
    Egg,
    Elder,
    Eldrazi,
    Elemental,
    Elephant,
    Elf,
    Elk,
    Ellywick,
    Elminster,
    Elspeth,
    Employee,
    Equipment,
    Estrid,
    Eye,
    Faerie,
    Ferret,
    Fish,
    Flagbearer,
    Food,
    Forest,
    Fortification,
    Fox,
    Fractal,
    Freyalise,
    Frog,
    Fungus,
    Gamer,
    Gargoyle,
    Garruk,
    Gate,
    Germ,
    Giant,
    Gideon,
    Gith,
    Gnoll,
    Gnome,
    Goat,
    Goblin,
    God,
    Golem,
    Gorgon,
    Graveborn,
    Gremlin,
    Griffin,
    Grist,
    Guest,
    Guff,
    Hag,
    Halfling,
    Hamster,
    Harpy,
    Hellion,
    Hippo,
    Hippogriff,
    Homarid,
    Homunculus,
    Horror,
    Horse,
    Huatli,
    Human,
    Hydra,
    Hyena,
    Illusion,
    Imp,
    Incarnation,
    Inkling,
    Inquisitor,
    Insect,
    Island,
    Jace,
    Jackal,
    Jared,
    Jaya,
    Jellyfish,
    Jeska,
    Juggernaut,
    Kaito,
    Karn,
    Kasmina,
    Kavu,
    Kaya,
    Kiora,
    Kirin,
    Kithkin,
    Knight,
    Kobold,
    Kor,
    Koth,
    Kraken,
    Lair,
    Lamia,
    Lammasu,
    Leech,
    Lesson,
    Leviathan,
    Lhurgoyf,
    Licid,
    Liliana,
    Lizard,
    Locus,
    Lolth,
    Lord,
    Lukka,
    Manticore,
    Masticore,
    Mercenary,
    Merfolk,
    Metathran,
    Mine,
    Minion,
    Minotaur,
    Minsc,
    Mite,
    Mole,
    Monger,
    Mongoose,
    Monk,
    Monkey,
    Moonfolk,
    Mordenkainen,
    Mountain,
    Mouse,
    Mutant,
    Myr,
    Mystic,
    Naga,
    Nahiri,
    Narset,
    Nautilus,
    Necron,
    Nephilim,
    Nightmare,
    Nightstalker,
    Niko,
    Ninja,
    Nissa,
    Nixilis,
    Noble,
    Noggle,
    Nomad,
    Nymph,
    Octopus,
    Ogre,
    Oko,
    Ooze,
    Orb,
    Orc,
    Orgg,
    Otter,
    Ouphe,
    Ox,
    Oyster,
    Pangolin,
    Peasant,
    Pegasus,
    Pentavite,
    Performer,
    Pest,
    Phelddagrif,
    Phoenix,
    Phyrexian,
    Pilot,
    Pincher,
    Pirate,
    Plains,
    Plant,
    PowerPlant,
    Powerstone,
    Praetor,
    Primarch,
    Prism,
    Processor,
    Quintorius,
    Rabbit,
    Raccoon,
    Ral,
    Ranger,
    Rat,
    Rebel,
    Reflection,
    Rhino,
    Rigger,
    Robot,
    Rogue,
    Rowan,
    Rune,
    Sable,
    Saga,
    Saheeli,
    Salamander,
    Samurai,
    Sand,
    Saproling,
    Samut,
    Sarkhan,
    Satyr,
    Scarecrow,
    Scientist,
    Scion,
    Scorpion,
    Scout,
    Sculpture,
    Serf,
    Serpent,
    Servo,
    Serra,
    Shade,
    Shaman,
    Shapeshifter,
    Shark,
    Sheep,
    Shrine,
    Siege,
    Siren,
    Sivitri,
    Skeleton,
    Slith,
    Sliver,
    Slug,
    Snail,
    Snake,
    Soldier,
    Soltari,
    Sorin,
    Spawn,
    Specter,
    Spellshaper,
    Sphere,
    Sphinx,
    Spider,
    Spike,
    Spirit,
    Splinter,
    Sponge,
    Squid,
    Squirrel,
    Starfish,
    Surrakar,
    Survivor,
    Swamp,
    Szat,
    Tamiyo,
    Tasha,
    Teferi,
    Tentacle,
    Tetravite,
    Teyo,
    Tezzeret,
    Thalakos,
    Thopter,
    Thrull,
    Tibalt,
    Tiefling,
    Time,
    Tower,
    Trap,
    Treasure,
    Treefolk,
    Trilobite,
    Triskelavite,
    Troll,
    Turtle,
    Tyranid,
    Tyvar,
    Ugin,
    Unicorn,
    Urza,
    Urzas,
    Vampire,
    Vedalken,
    Vehicle,
    Venser,
    Viashino,
    Vivien,
    Volver,
    Vraska,
    Vronos,
    Wall,
    Walrus,
    Warlock,
    Warrior,
    Weird,
    Werewolf,
    Whale,
    Will,
    Windgrace,
    Wizard,
    Wolf,
    Wolverine,
    Wombat,
    Worm,
    Wraith,
    Wrenn,
    Wurm,
    Xenagos,
    Yanggu,
    Yanling,
    Yeti,
    Zariel,
    Zombie,
    Zubera,
}

impl Subtype {
    pub(crate) fn is_creature_type(&self) -> bool {
        matches!(
            self,
            Self::Advisor
                | Self::Aetherborn
                | Self::Alien
                | Self::Ally
                | Self::Angel
                | Self::Antelope
                | Self::Ape
                | Self::Archer
                | Self::Archon
                | Self::Army
                | Self::Artificer
                | Self::Assassin
                | Self::AssemblyWorker
                | Self::Astartes
                | Self::Atog
                | Self::Aurochs
                | Self::Avatar
                | Self::Azra
                | Self::Badger
                | Self::Balloon
                | Self::Barbarian
                | Self::Bard
                | Self::Basilisk
                | Self::Bat
                | Self::Bear
                | Self::Beast
                | Self::Beeble
                | Self::Beholder
                | Self::Berserker
                | Self::Bird
                | Self::Blinkmoth
                | Self::Boar
                | Self::Bringer
                | Self::Brushwagg
                | Self::Camarid
                | Self::Camel
                | Self::Caribou
                | Self::Carrier
                | Self::Cat
                | Self::Centaur
                | Self::Cephalid
                | Self::Child
                | Self::Chimera
                | Self::Citizen
                | Self::Cleric
                | Self::Clown
                | Self::Cockatrice
                | Self::Construct
                | Self::Coward
                | Self::Crab
                | Self::Crocodile
                | Self::Ctan
                | Self::Custodes
                | Self::Cyberman
                | Self::Cyclops
                | Self::Dalek
                | Self::Dauthi
                | Self::Demigod
                | Self::Demon
                | Self::Deserter
                | Self::Detective
                | Self::Devil
                | Self::Dinosaur
                | Self::Djinn
                | Self::Doctor
                | Self::Dog
                | Self::Dragon
                | Self::Drake
                | Self::Dreadnought
                | Self::Drone
                | Self::Druid
                | Self::Dryad
                | Self::Dwarf
                | Self::Efreet
                | Self::Egg
                | Self::Elder
                | Self::Eldrazi
                | Self::Elemental
                | Self::Elephant
                | Self::Elf
                | Self::Elk
                | Self::Employee
                | Self::Eye
                | Self::Faerie
                | Self::Ferret
                | Self::Fish
                | Self::Flagbearer
                | Self::Fox
                | Self::Fractal
                | Self::Frog
                | Self::Fungus
                | Self::Gamer
                | Self::Gargoyle
                | Self::Germ
                | Self::Giant
                | Self::Gith
                | Self::Gnoll
                | Self::Gnome
                | Self::Goat
                | Self::Goblin
                | Self::God
                | Self::Golem
                | Self::Gorgon
                | Self::Graveborn
                | Self::Gremlin
                | Self::Griffin
                | Self::Guest
                | Self::Hag
                | Self::Halfling
                | Self::Hamster
                | Self::Harpy
                | Self::Hellion
                | Self::Hippo
                | Self::Hippogriff
                | Self::Homarid
                | Self::Homunculus
                | Self::Horror
                | Self::Horse
                | Self::Human
                | Self::Hydra
                | Self::Hyena
                | Self::Illusion
                | Self::Imp
                | Self::Incarnation
                | Self::Inkling
                | Self::Inquisitor
                | Self::Insect
                | Self::Jackal
                | Self::Jellyfish
                | Self::Juggernaut
                | Self::Kavu
                | Self::Kirin
                | Self::Kithkin
                | Self::Knight
                | Self::Kobold
                | Self::Kor
                | Self::Kraken
                | Self::Lamia
                | Self::Lammasu
                | Self::Leech
                | Self::Leviathan
                | Self::Lhurgoyf
                | Self::Licid
                | Self::Lizard
                | Self::Lord
                | Self::Manticore
                | Self::Masticore
                | Self::Mercenary
                | Self::Merfolk
                | Self::Metathran
                | Self::Minion
                | Self::Minotaur
                | Self::Mite
                | Self::Mole
                | Self::Monger
                | Self::Mongoose
                | Self::Monk
                | Self::Monkey
                | Self::Moonfolk
                | Self::Mouse
                | Self::Mutant
                | Self::Myr
                | Self::Mystic
                | Self::Naga
                | Self::Nautilus
                | Self::Necron
                | Self::Nephilim
                | Self::Nightmare
                | Self::Nightstalker
                | Self::Ninja
                | Self::Noble
                | Self::Noggle
                | Self::Nomad
                | Self::Nymph
                | Self::Octopus
                | Self::Ogre
                | Self::Ooze
                | Self::Orb
                | Self::Orc
                | Self::Orgg
                | Self::Otter
                | Self::Ouphe
                | Self::Ox
                | Self::Oyster
                | Self::Pangolin
                | Self::Peasant
                | Self::Pegasus
                | Self::Pentavite
                | Self::Performer
                | Self::Pest
                | Self::Phelddagrif
                | Self::Phoenix
                | Self::Phyrexian
                | Self::Pilot
                | Self::Pincher
                | Self::Pirate
                | Self::Plant
                | Self::Praetor
                | Self::Primarch
                | Self::Prism
                | Self::Processor
                | Self::Raccoon
                | Self::Rabbit
                | Self::Ranger
                | Self::Rat
                | Self::Rebel
                | Self::Reflection
                | Self::Rhino
                | Self::Rigger
                | Self::Robot
                | Self::Rogue
                | Self::Sable
                | Self::Salamander
                | Self::Samurai
                | Self::Sand
                | Self::Saproling
                | Self::Satyr
                | Self::Scarecrow
                | Self::Scientist
                | Self::Scion
                | Self::Scorpion
                | Self::Scout
                | Self::Sculpture
                | Self::Serf
                | Self::Serpent
                | Self::Servo
                | Self::Shade
                | Self::Shaman
                | Self::Shapeshifter
                | Self::Shark
                | Self::Sheep
                | Self::Siren
                | Self::Skeleton
                | Self::Slith
                | Self::Sliver
                | Self::Slug
                | Self::Snake
                | Self::Soldier
                | Self::Soltari
                | Self::Spawn
                | Self::Specter
                | Self::Spellshaper
                | Self::Sphinx
                | Self::Spider
                | Self::Spike
                | Self::Spirit
                | Self::Splinter
                | Self::Sponge
                | Self::Squid
                | Self::Squirrel
                | Self::Starfish
                | Self::Surrakar
                | Self::Survivor
                | Self::Tentacle
                | Self::Tetravite
                | Self::Thalakos
                | Self::Thopter
                | Self::Thrull
                | Self::Tiefling
                | Self::Time
                | Self::Treefolk
                | Self::Trilobite
                | Self::Triskelavite
                | Self::Troll
                | Self::Turtle
                | Self::Tyranid
                | Self::Unicorn
                | Self::Vampire
                | Self::Vedalken
                | Self::Viashino
                | Self::Volver
                | Self::Wall
                | Self::Walrus
                | Self::Warlock
                | Self::Warrior
                | Self::Weird
                | Self::Werewolf
                | Self::Whale
                | Self::Wizard
                | Self::Wolf
                | Self::Wolverine
                | Self::Wombat
                | Self::Worm
                | Self::Wraith
                | Self::Wurm
                | Self::Yeti
                | Self::Zombie
                | Self::Zubera
        )
    }
}

impl TryFrom<&protogen::types::Subtype> for Subtype {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::types::Subtype) -> Result<Self, Self::Error> {
        value
            .subtype
            .as_ref()
            .ok_or_else(|| anyhow!("Expected subtype to have a subtype specified"))
            .map(Subtype::from)
    }
}

impl From<&protogen::types::subtype::Subtype> for Subtype {
    fn from(value: &protogen::types::subtype::Subtype) -> Self {
        match value {
            protogen::types::subtype::Subtype::Adventure(_) => Self::Adventure,
            protogen::types::subtype::Subtype::Advisor(_) => Self::Advisor,
            protogen::types::subtype::Subtype::Aetherborn(_) => Self::Aetherborn,
            protogen::types::subtype::Subtype::Ajani(_) => Self::Ajani,
            protogen::types::subtype::Subtype::Alien(_) => Self::Alien,
            protogen::types::subtype::Subtype::Ally(_) => Self::Ally,
            protogen::types::subtype::Subtype::Aminatou(_) => Self::Aminatou,
            protogen::types::subtype::Subtype::Angel(_) => Self::Angel,
            protogen::types::subtype::Subtype::Angrath(_) => Self::Angrath,
            protogen::types::subtype::Subtype::Antelope(_) => Self::Antelope,
            protogen::types::subtype::Subtype::Ape(_) => Self::Ape,
            protogen::types::subtype::Subtype::Arcane(_) => Self::Arcane,
            protogen::types::subtype::Subtype::Archer(_) => Self::Archer,
            protogen::types::subtype::Subtype::Archon(_) => Self::Archon,
            protogen::types::subtype::Subtype::Arlinn(_) => Self::Arlinn,
            protogen::types::subtype::Subtype::Artificer(_) => Self::Artificer,
            protogen::types::subtype::Subtype::Ashiok(_) => Self::Ashiok,
            protogen::types::subtype::Subtype::Assassin(_) => Self::Assassin,
            protogen::types::subtype::Subtype::AssemblyWorker(_) => Self::AssemblyWorker,
            protogen::types::subtype::Subtype::Astartes(_) => Self::Astartes,
            protogen::types::subtype::Subtype::Atog(_) => Self::Atog,
            protogen::types::subtype::Subtype::Attraction(_) => Self::Attraction,
            protogen::types::subtype::Subtype::Aura(_) => Self::Aura,
            protogen::types::subtype::Subtype::Aurochs(_) => Self::Aurochs,
            protogen::types::subtype::Subtype::Avatar(_) => Self::Avatar,
            protogen::types::subtype::Subtype::Azra(_) => Self::Azra,
            protogen::types::subtype::Subtype::Background(_) => Self::Background,
            protogen::types::subtype::Subtype::Badger(_) => Self::Badger,
            protogen::types::subtype::Subtype::Bahamut(_) => Self::Bahamut,
            protogen::types::subtype::Subtype::Barbarian(_) => Self::Barbarian,
            protogen::types::subtype::Subtype::Bard(_) => Self::Bard,
            protogen::types::subtype::Subtype::Basilisk(_) => Self::Basilisk,
            protogen::types::subtype::Subtype::Basri(_) => Self::Basri,
            protogen::types::subtype::Subtype::Bat(_) => Self::Bat,
            protogen::types::subtype::Subtype::Bear(_) => Self::Bear,
            protogen::types::subtype::Subtype::Beast(_) => Self::Beast,
            protogen::types::subtype::Subtype::Beeble(_) => Self::Beeble,
            protogen::types::subtype::Subtype::Beholder(_) => Self::Beholder,
            protogen::types::subtype::Subtype::Berserker(_) => Self::Berserker,
            protogen::types::subtype::Subtype::Bird(_) => Self::Bird,
            protogen::types::subtype::Subtype::Boar(_) => Self::Boar,
            protogen::types::subtype::Subtype::Bolas(_) => Self::Bolas,
            protogen::types::subtype::Subtype::Bringer(_) => Self::Bringer,
            protogen::types::subtype::Subtype::Brushwagg(_) => Self::Brushwagg,
            protogen::types::subtype::Subtype::Calix(_) => Self::Calix,
            protogen::types::subtype::Subtype::Camel(_) => Self::Camel,
            protogen::types::subtype::Subtype::Capybara(_) => Self::Capybara,
            protogen::types::subtype::Subtype::Carrier(_) => Self::Carrier,
            protogen::types::subtype::Subtype::Cartouche(_) => Self::Cartouche,
            protogen::types::subtype::Subtype::Cat(_) => Self::Cat,
            protogen::types::subtype::Subtype::Cave(_) => Self::Cave,
            protogen::types::subtype::Subtype::Centaur(_) => Self::Centaur,
            protogen::types::subtype::Subtype::Cephalid(_) => Self::Cephalid,
            protogen::types::subtype::Subtype::Chandra(_) => Self::Chandra,
            protogen::types::subtype::Subtype::Child(_) => Self::Child,
            protogen::types::subtype::Subtype::Chimera(_) => Self::Chimera,
            protogen::types::subtype::Subtype::Citizen(_) => Self::Citizen,
            protogen::types::subtype::Subtype::Class(_) => Self::Class,
            protogen::types::subtype::Subtype::Cleric(_) => Self::Cleric,
            protogen::types::subtype::Subtype::Clown(_) => Self::Clown,
            protogen::types::subtype::Subtype::Clue(_) => Self::Clue,
            protogen::types::subtype::Subtype::Cockatrice(_) => Self::Cockatrice,
            protogen::types::subtype::Subtype::Comet(_) => Self::Comet,
            protogen::types::subtype::Subtype::Construct(_) => Self::Construct,
            protogen::types::subtype::Subtype::Coward(_) => Self::Coward,
            protogen::types::subtype::Subtype::Crab(_) => Self::Crab,
            protogen::types::subtype::Subtype::Crocodile(_) => Self::Crocodile,
            protogen::types::subtype::Subtype::Ctan(_) => Self::Ctan,
            protogen::types::subtype::Subtype::Curse(_) => Self::Curse,
            protogen::types::subtype::Subtype::Custodes(_) => Self::Custodes,
            protogen::types::subtype::Subtype::Cyberman(_) => Self::Cyberman,
            protogen::types::subtype::Subtype::Cyclops(_) => Self::Cyclops,
            protogen::types::subtype::Subtype::Dack(_) => Self::Dack,
            protogen::types::subtype::Subtype::Dakkon(_) => Self::Dakkon,
            protogen::types::subtype::Subtype::Dalek(_) => Self::Dalek,
            protogen::types::subtype::Subtype::Daretti(_) => Self::Daretti,
            protogen::types::subtype::Subtype::Dauthi(_) => Self::Dauthi,
            protogen::types::subtype::Subtype::Davriel(_) => Self::Davriel,
            protogen::types::subtype::Subtype::Demigod(_) => Self::Demigod,
            protogen::types::subtype::Subtype::Demon(_) => Self::Demon,
            protogen::types::subtype::Subtype::Desert(_) => Self::Desert,
            protogen::types::subtype::Subtype::Detective(_) => Self::Detective,
            protogen::types::subtype::Subtype::Devil(_) => Self::Devil,
            protogen::types::subtype::Subtype::Dihada(_) => Self::Dihada,
            protogen::types::subtype::Subtype::Dinosaur(_) => Self::Dinosaur,
            protogen::types::subtype::Subtype::Djinn(_) => Self::Djinn,
            protogen::types::subtype::Subtype::Doctor(_) => Self::Doctor,
            protogen::types::subtype::Subtype::Dog(_) => Self::Dog,
            protogen::types::subtype::Subtype::Domri(_) => Self::Domri,
            protogen::types::subtype::Subtype::Dovin(_) => Self::Dovin,
            protogen::types::subtype::Subtype::Dragon(_) => Self::Dragon,
            protogen::types::subtype::Subtype::Drake(_) => Self::Drake,
            protogen::types::subtype::Subtype::Dreadnought(_) => Self::Dreadnought,
            protogen::types::subtype::Subtype::Drone(_) => Self::Drone,
            protogen::types::subtype::Subtype::Druid(_) => Self::Druid,
            protogen::types::subtype::Subtype::Dryad(_) => Self::Dryad,
            protogen::types::subtype::Subtype::Dwarf(_) => Self::Dwarf,
            protogen::types::subtype::Subtype::Efreet(_) => Self::Efreet,
            protogen::types::subtype::Subtype::Egg(_) => Self::Egg,
            protogen::types::subtype::Subtype::Elder(_) => Self::Elder,
            protogen::types::subtype::Subtype::Eldrazi(_) => Self::Eldrazi,
            protogen::types::subtype::Subtype::Elemental(_) => Self::Elemental,
            protogen::types::subtype::Subtype::Elephant(_) => Self::Elephant,
            protogen::types::subtype::Subtype::Elf(_) => Self::Elf,
            protogen::types::subtype::Subtype::Elk(_) => Self::Elk,
            protogen::types::subtype::Subtype::Ellywick(_) => Self::Ellywick,
            protogen::types::subtype::Subtype::Elminster(_) => Self::Elminster,
            protogen::types::subtype::Subtype::Elspeth(_) => Self::Elspeth,
            protogen::types::subtype::Subtype::Employee(_) => Self::Employee,
            protogen::types::subtype::Subtype::Equipment(_) => Self::Equipment,
            protogen::types::subtype::Subtype::Estrid(_) => Self::Estrid,
            protogen::types::subtype::Subtype::Eye(_) => Self::Eye,
            protogen::types::subtype::Subtype::Faerie(_) => Self::Faerie,
            protogen::types::subtype::Subtype::Ferret(_) => Self::Ferret,
            protogen::types::subtype::Subtype::Fish(_) => Self::Fish,
            protogen::types::subtype::Subtype::Flagbearer(_) => Self::Flagbearer,
            protogen::types::subtype::Subtype::Food(_) => Self::Food,
            protogen::types::subtype::Subtype::Forest(_) => Self::Forest,
            protogen::types::subtype::Subtype::Fortification(_) => Self::Fortification,
            protogen::types::subtype::Subtype::Fox(_) => Self::Fox,
            protogen::types::subtype::Subtype::Fractal(_) => Self::Fractal,
            protogen::types::subtype::Subtype::Freyalise(_) => Self::Freyalise,
            protogen::types::subtype::Subtype::Frog(_) => Self::Frog,
            protogen::types::subtype::Subtype::Fungus(_) => Self::Fungus,
            protogen::types::subtype::Subtype::Gamer(_) => Self::Gamer,
            protogen::types::subtype::Subtype::Gargoyle(_) => Self::Gargoyle,
            protogen::types::subtype::Subtype::Garruk(_) => Self::Garruk,
            protogen::types::subtype::Subtype::Gate(_) => Self::Gate,
            protogen::types::subtype::Subtype::Giant(_) => Self::Giant,
            protogen::types::subtype::Subtype::Gideon(_) => Self::Gideon,
            protogen::types::subtype::Subtype::Gith(_) => Self::Gith,
            protogen::types::subtype::Subtype::Gnoll(_) => Self::Gnoll,
            protogen::types::subtype::Subtype::Gnome(_) => Self::Gnome,
            protogen::types::subtype::Subtype::Goat(_) => Self::Goat,
            protogen::types::subtype::Subtype::Goblin(_) => Self::Goblin,
            protogen::types::subtype::Subtype::God(_) => Self::God,
            protogen::types::subtype::Subtype::Golem(_) => Self::Golem,
            protogen::types::subtype::Subtype::Gorgon(_) => Self::Gorgon,
            protogen::types::subtype::Subtype::Gremlin(_) => Self::Gremlin,
            protogen::types::subtype::Subtype::Griffin(_) => Self::Griffin,
            protogen::types::subtype::Subtype::Grist(_) => Self::Grist,
            protogen::types::subtype::Subtype::Guest(_) => Self::Guest,
            protogen::types::subtype::Subtype::Guff(_) => Self::Guff,
            protogen::types::subtype::Subtype::Hag(_) => Self::Hag,
            protogen::types::subtype::Subtype::Halfling(_) => Self::Halfling,
            protogen::types::subtype::Subtype::Harpy(_) => Self::Harpy,
            protogen::types::subtype::Subtype::Hellion(_) => Self::Hellion,
            protogen::types::subtype::Subtype::Hippo(_) => Self::Hippo,
            protogen::types::subtype::Subtype::Hippogriff(_) => Self::Hippogriff,
            protogen::types::subtype::Subtype::Homarid(_) => Self::Homarid,
            protogen::types::subtype::Subtype::Homunculus(_) => Self::Homunculus,
            protogen::types::subtype::Subtype::Horror(_) => Self::Horror,
            protogen::types::subtype::Subtype::Horse(_) => Self::Horse,
            protogen::types::subtype::Subtype::Huatli(_) => Self::Huatli,
            protogen::types::subtype::Subtype::Human(_) => Self::Human,
            protogen::types::subtype::Subtype::Hydra(_) => Self::Hydra,
            protogen::types::subtype::Subtype::Hyena(_) => Self::Hyena,
            protogen::types::subtype::Subtype::Illusion(_) => Self::Illusion,
            protogen::types::subtype::Subtype::Imp(_) => Self::Imp,
            protogen::types::subtype::Subtype::Incarnation(_) => Self::Incarnation,
            protogen::types::subtype::Subtype::Inquisitor(_) => Self::Inquisitor,
            protogen::types::subtype::Subtype::Insect(_) => Self::Insect,
            protogen::types::subtype::Subtype::Island(_) => Self::Island,
            protogen::types::subtype::Subtype::Jace(_) => Self::Jace,
            protogen::types::subtype::Subtype::Jackal(_) => Self::Jackal,
            protogen::types::subtype::Subtype::Jared(_) => Self::Jared,
            protogen::types::subtype::Subtype::Jaya(_) => Self::Jaya,
            protogen::types::subtype::Subtype::Jellyfish(_) => Self::Jellyfish,
            protogen::types::subtype::Subtype::Jeska(_) => Self::Jeska,
            protogen::types::subtype::Subtype::Juggernaut(_) => Self::Juggernaut,
            protogen::types::subtype::Subtype::Kaito(_) => Self::Kaito,
            protogen::types::subtype::Subtype::Karn(_) => Self::Karn,
            protogen::types::subtype::Subtype::Kasmina(_) => Self::Kasmina,
            protogen::types::subtype::Subtype::Kavu(_) => Self::Kavu,
            protogen::types::subtype::Subtype::Kaya(_) => Self::Kaya,
            protogen::types::subtype::Subtype::Kiora(_) => Self::Kiora,
            protogen::types::subtype::Subtype::Kirin(_) => Self::Kirin,
            protogen::types::subtype::Subtype::Kithkin(_) => Self::Kithkin,
            protogen::types::subtype::Subtype::Knight(_) => Self::Knight,
            protogen::types::subtype::Subtype::Kobold(_) => Self::Kobold,
            protogen::types::subtype::Subtype::Kor(_) => Self::Kor,
            protogen::types::subtype::Subtype::Koth(_) => Self::Koth,
            protogen::types::subtype::Subtype::Kraken(_) => Self::Kraken,
            protogen::types::subtype::Subtype::Lair(_) => Self::Lair,
            protogen::types::subtype::Subtype::Lamia(_) => Self::Lamia,
            protogen::types::subtype::Subtype::Lammasu(_) => Self::Lammasu,
            protogen::types::subtype::Subtype::Leech(_) => Self::Leech,
            protogen::types::subtype::Subtype::Lesson(_) => Self::Lesson,
            protogen::types::subtype::Subtype::Leviathan(_) => Self::Leviathan,
            protogen::types::subtype::Subtype::Lhurgoyf(_) => Self::Lhurgoyf,
            protogen::types::subtype::Subtype::Licid(_) => Self::Licid,
            protogen::types::subtype::Subtype::Liliana(_) => Self::Liliana,
            protogen::types::subtype::Subtype::Lizard(_) => Self::Lizard,
            protogen::types::subtype::Subtype::Locus(_) => Self::Locus,
            protogen::types::subtype::Subtype::Lolth(_) => Self::Lolth,
            protogen::types::subtype::Subtype::Lord(_) => Self::Lord,
            protogen::types::subtype::Subtype::Lukka(_) => Self::Lukka,
            protogen::types::subtype::Subtype::Manticore(_) => Self::Manticore,
            protogen::types::subtype::Subtype::Masticore(_) => Self::Masticore,
            protogen::types::subtype::Subtype::Mercenary(_) => Self::Mercenary,
            protogen::types::subtype::Subtype::Merfolk(_) => Self::Merfolk,
            protogen::types::subtype::Subtype::Metathran(_) => Self::Metathran,
            protogen::types::subtype::Subtype::Mine(_) => Self::Mine,
            protogen::types::subtype::Subtype::Minion(_) => Self::Minion,
            protogen::types::subtype::Subtype::Minotaur(_) => Self::Minotaur,
            protogen::types::subtype::Subtype::Minsc(_) => Self::Minsc,
            protogen::types::subtype::Subtype::Mite(_) => Self::Mite,
            protogen::types::subtype::Subtype::Mole(_) => Self::Mole,
            protogen::types::subtype::Subtype::Monger(_) => Self::Monger,
            protogen::types::subtype::Subtype::Mongoose(_) => Self::Mongoose,
            protogen::types::subtype::Subtype::Monk(_) => Self::Monk,
            protogen::types::subtype::Subtype::Monkey(_) => Self::Monkey,
            protogen::types::subtype::Subtype::Moonfolk(_) => Self::Moonfolk,
            protogen::types::subtype::Subtype::Mordenkainen(_) => Self::Mordenkainen,
            protogen::types::subtype::Subtype::Mountain(_) => Self::Mountain,
            protogen::types::subtype::Subtype::Mouse(_) => Self::Mouse,
            protogen::types::subtype::Subtype::Mutant(_) => Self::Mutant,
            protogen::types::subtype::Subtype::Myr(_) => Self::Myr,
            protogen::types::subtype::Subtype::Mystic(_) => Self::Mystic,
            protogen::types::subtype::Subtype::Naga(_) => Self::Naga,
            protogen::types::subtype::Subtype::Nahiri(_) => Self::Nahiri,
            protogen::types::subtype::Subtype::Narset(_) => Self::Narset,
            protogen::types::subtype::Subtype::Nautilus(_) => Self::Nautilus,
            protogen::types::subtype::Subtype::Necron(_) => Self::Necron,
            protogen::types::subtype::Subtype::Nephilim(_) => Self::Nephilim,
            protogen::types::subtype::Subtype::Nightmare(_) => Self::Nightmare,
            protogen::types::subtype::Subtype::Nightstalker(_) => Self::Nightstalker,
            protogen::types::subtype::Subtype::Niko(_) => Self::Niko,
            protogen::types::subtype::Subtype::Ninja(_) => Self::Ninja,
            protogen::types::subtype::Subtype::Nissa(_) => Self::Nissa,
            protogen::types::subtype::Subtype::Nixilis(_) => Self::Nixilis,
            protogen::types::subtype::Subtype::Noble(_) => Self::Noble,
            protogen::types::subtype::Subtype::Noggle(_) => Self::Noggle,
            protogen::types::subtype::Subtype::Nomad(_) => Self::Nomad,
            protogen::types::subtype::Subtype::Nymph(_) => Self::Nymph,
            protogen::types::subtype::Subtype::Octopus(_) => Self::Octopus,
            protogen::types::subtype::Subtype::Ogre(_) => Self::Ogre,
            protogen::types::subtype::Subtype::Oko(_) => Self::Oko,
            protogen::types::subtype::Subtype::Ooze(_) => Self::Ooze,
            protogen::types::subtype::Subtype::Orc(_) => Self::Orc,
            protogen::types::subtype::Subtype::Orgg(_) => Self::Orgg,
            protogen::types::subtype::Subtype::Otter(_) => Self::Otter,
            protogen::types::subtype::Subtype::Ouphe(_) => Self::Ouphe,
            protogen::types::subtype::Subtype::Ox(_) => Self::Ox,
            protogen::types::subtype::Subtype::Oyster(_) => Self::Oyster,
            protogen::types::subtype::Subtype::Pangolin(_) => Self::Pangolin,
            protogen::types::subtype::Subtype::Peasant(_) => Self::Peasant,
            protogen::types::subtype::Subtype::Pegasus(_) => Self::Pegasus,
            protogen::types::subtype::Subtype::Performer(_) => Self::Performer,
            protogen::types::subtype::Subtype::Pest(_) => Self::Pest,
            protogen::types::subtype::Subtype::Phelddagrif(_) => Self::Phelddagrif,
            protogen::types::subtype::Subtype::Phoenix(_) => Self::Phoenix,
            protogen::types::subtype::Subtype::Phyrexian(_) => Self::Phyrexian,
            protogen::types::subtype::Subtype::Pilot(_) => Self::Pilot,
            protogen::types::subtype::Subtype::Pirate(_) => Self::Pirate,
            protogen::types::subtype::Subtype::Plains(_) => Self::Plains,
            protogen::types::subtype::Subtype::Plant(_) => Self::Plant,
            protogen::types::subtype::Subtype::PowerPlant(_) => Self::PowerPlant,
            protogen::types::subtype::Subtype::Powerstone(_) => Self::Powerstone,
            protogen::types::subtype::Subtype::Praetor(_) => Self::Praetor,
            protogen::types::subtype::Subtype::Primarch(_) => Self::Primarch,
            protogen::types::subtype::Subtype::Processor(_) => Self::Processor,
            protogen::types::subtype::Subtype::Quintorius(_) => Self::Quintorius,
            protogen::types::subtype::Subtype::Rabbit(_) => Self::Rabbit,
            protogen::types::subtype::Subtype::Raccoon(_) => Self::Raccoon,
            protogen::types::subtype::Subtype::Ral(_) => Self::Ral,
            protogen::types::subtype::Subtype::Ranger(_) => Self::Ranger,
            protogen::types::subtype::Subtype::Rat(_) => Self::Rat,
            protogen::types::subtype::Subtype::Rebel(_) => Self::Rebel,
            protogen::types::subtype::Subtype::Rhino(_) => Self::Rhino,
            protogen::types::subtype::Subtype::Rigger(_) => Self::Rigger,
            protogen::types::subtype::Subtype::Robot(_) => Self::Robot,
            protogen::types::subtype::Subtype::Rogue(_) => Self::Rogue,
            protogen::types::subtype::Subtype::Rowan(_) => Self::Rowan,
            protogen::types::subtype::Subtype::Rune(_) => Self::Rune,
            protogen::types::subtype::Subtype::Sable(_) => Self::Sable,
            protogen::types::subtype::Subtype::Saga(_) => Self::Saga,
            protogen::types::subtype::Subtype::Saheeli(_) => Self::Saheeli,
            protogen::types::subtype::Subtype::Salamander(_) => Self::Salamander,
            protogen::types::subtype::Subtype::Samurai(_) => Self::Samurai,
            protogen::types::subtype::Subtype::Samut(_) => Self::Samut,
            protogen::types::subtype::Subtype::Sarkhan(_) => Self::Sarkhan,
            protogen::types::subtype::Subtype::Satyr(_) => Self::Satyr,
            protogen::types::subtype::Subtype::Scarecrow(_) => Self::Scarecrow,
            protogen::types::subtype::Subtype::Scientist(_) => Self::Scientist,
            protogen::types::subtype::Subtype::Scorpion(_) => Self::Scorpion,
            protogen::types::subtype::Subtype::Scout(_) => Self::Scout,
            protogen::types::subtype::Subtype::Serpent(_) => Self::Serpent,
            protogen::types::subtype::Subtype::Serra(_) => Self::Serra,
            protogen::types::subtype::Subtype::Shade(_) => Self::Shade,
            protogen::types::subtype::Subtype::Shaman(_) => Self::Shaman,
            protogen::types::subtype::Subtype::Shapeshifter(_) => Self::Shapeshifter,
            protogen::types::subtype::Subtype::Shark(_) => Self::Shark,
            protogen::types::subtype::Subtype::Sheep(_) => Self::Sheep,
            protogen::types::subtype::Subtype::Shrine(_) => Self::Shrine,
            protogen::types::subtype::Subtype::Siege(_) => Self::Siege,
            protogen::types::subtype::Subtype::Siren(_) => Self::Siren,
            protogen::types::subtype::Subtype::Sivitri(_) => Self::Sivitri,
            protogen::types::subtype::Subtype::Skeleton(_) => Self::Skeleton,
            protogen::types::subtype::Subtype::Slith(_) => Self::Slith,
            protogen::types::subtype::Subtype::Sliver(_) => Self::Sliver,
            protogen::types::subtype::Subtype::Slug(_) => Self::Slug,
            protogen::types::subtype::Subtype::Snail(_) => Self::Snail,
            protogen::types::subtype::Subtype::Snake(_) => Self::Snake,
            protogen::types::subtype::Subtype::Soldier(_) => Self::Soldier,
            protogen::types::subtype::Subtype::Soltari(_) => Self::Soltari,
            protogen::types::subtype::Subtype::Sorin(_) => Self::Sorin,
            protogen::types::subtype::Subtype::Spawn(_) => Self::Spawn,
            protogen::types::subtype::Subtype::Specter(_) => Self::Specter,
            protogen::types::subtype::Subtype::Spellshaper(_) => Self::Spellshaper,
            protogen::types::subtype::Subtype::Sphere(_) => Self::Sphere,
            protogen::types::subtype::Subtype::Sphinx(_) => Self::Sphinx,
            protogen::types::subtype::Subtype::Spider(_) => Self::Spider,
            protogen::types::subtype::Subtype::Spike(_) => Self::Spike,
            protogen::types::subtype::Subtype::Spirit(_) => Self::Spirit,
            protogen::types::subtype::Subtype::Sponge(_) => Self::Sponge,
            protogen::types::subtype::Subtype::Squid(_) => Self::Squid,
            protogen::types::subtype::Subtype::Squirrel(_) => Self::Squirrel,
            protogen::types::subtype::Subtype::Starfish(_) => Self::Starfish,
            protogen::types::subtype::Subtype::Surrakar(_) => Self::Surrakar,
            protogen::types::subtype::Subtype::Swamp(_) => Self::Swamp,
            protogen::types::subtype::Subtype::Szat(_) => Self::Szat,
            protogen::types::subtype::Subtype::Tamiyo(_) => Self::Tamiyo,
            protogen::types::subtype::Subtype::Tasha(_) => Self::Tasha,
            protogen::types::subtype::Subtype::Teferi(_) => Self::Teferi,
            protogen::types::subtype::Subtype::Teyo(_) => Self::Teyo,
            protogen::types::subtype::Subtype::Tezzeret(_) => Self::Tezzeret,
            protogen::types::subtype::Subtype::Thalakos(_) => Self::Thalakos,
            protogen::types::subtype::Subtype::Thopter(_) => Self::Thopter,
            protogen::types::subtype::Subtype::Thrull(_) => Self::Thrull,
            protogen::types::subtype::Subtype::Tibalt(_) => Self::Tibalt,
            protogen::types::subtype::Subtype::Tiefling(_) => Self::Tiefling,
            protogen::types::subtype::Subtype::Time(_) => Self::Time,
            protogen::types::subtype::Subtype::Tower(_) => Self::Tower,
            protogen::types::subtype::Subtype::Trap(_) => Self::Trap,
            protogen::types::subtype::Subtype::Treasure(_) => Self::Treasure,
            protogen::types::subtype::Subtype::Treefolk(_) => Self::Treefolk,
            protogen::types::subtype::Subtype::Trilobite(_) => Self::Trilobite,
            protogen::types::subtype::Subtype::Troll(_) => Self::Troll,
            protogen::types::subtype::Subtype::Turtle(_) => Self::Turtle,
            protogen::types::subtype::Subtype::Tyranid(_) => Self::Tyranid,
            protogen::types::subtype::Subtype::Tyvar(_) => Self::Tyvar,
            protogen::types::subtype::Subtype::Ugin(_) => Self::Ugin,
            protogen::types::subtype::Subtype::Unicorn(_) => Self::Unicorn,
            protogen::types::subtype::Subtype::Urza(_) => Self::Urza,
            protogen::types::subtype::Subtype::Urzas(_) => Self::Urzas,
            protogen::types::subtype::Subtype::Vampire(_) => Self::Vampire,
            protogen::types::subtype::Subtype::Vedalken(_) => Self::Vedalken,
            protogen::types::subtype::Subtype::Vehicle(_) => Self::Vehicle,
            protogen::types::subtype::Subtype::Venser(_) => Self::Venser,
            protogen::types::subtype::Subtype::Viashino(_) => Self::Viashino,
            protogen::types::subtype::Subtype::Vivien(_) => Self::Vivien,
            protogen::types::subtype::Subtype::Volver(_) => Self::Volver,
            protogen::types::subtype::Subtype::Vraska(_) => Self::Vraska,
            protogen::types::subtype::Subtype::Vronos(_) => Self::Vronos,
            protogen::types::subtype::Subtype::Wall(_) => Self::Wall,
            protogen::types::subtype::Subtype::Walrus(_) => Self::Walrus,
            protogen::types::subtype::Subtype::Warlock(_) => Self::Warlock,
            protogen::types::subtype::Subtype::Warrior(_) => Self::Warrior,
            protogen::types::subtype::Subtype::Weird(_) => Self::Weird,
            protogen::types::subtype::Subtype::Werewolf(_) => Self::Werewolf,
            protogen::types::subtype::Subtype::Whale(_) => Self::Whale,
            protogen::types::subtype::Subtype::Will(_) => Self::Will,
            protogen::types::subtype::Subtype::Windgrace(_) => Self::Windgrace,
            protogen::types::subtype::Subtype::Wizard(_) => Self::Wizard,
            protogen::types::subtype::Subtype::Wolf(_) => Self::Wolf,
            protogen::types::subtype::Subtype::Wolverine(_) => Self::Wolverine,
            protogen::types::subtype::Subtype::Wombat(_) => Self::Wombat,
            protogen::types::subtype::Subtype::Worm(_) => Self::Worm,
            protogen::types::subtype::Subtype::Wraith(_) => Self::Wraith,
            protogen::types::subtype::Subtype::Wrenn(_) => Self::Wrenn,
            protogen::types::subtype::Subtype::Wurm(_) => Self::Wurm,
            protogen::types::subtype::Subtype::Xenagos(_) => Self::Xenagos,
            protogen::types::subtype::Subtype::Yanggu(_) => Self::Yanggu,
            protogen::types::subtype::Subtype::Yanling(_) => Self::Yanling,
            protogen::types::subtype::Subtype::Yeti(_) => Self::Yeti,
            protogen::types::subtype::Subtype::Zariel(_) => Self::Zariel,
            protogen::types::subtype::Subtype::Zombie(_) => Self::Zombie,
            protogen::types::subtype::Subtype::Zubera(_) => Self::Zubera,
        }
    }
}

pub(crate) fn parse_typeline(
    typeline: &str,
) -> anyhow::Result<(IndexSet<Type>, IndexSet<Subtype>)> {
    if typeline.is_empty() {
        return Err(anyhow!("Expected card to have types set"));
    }

    let types_and_subtypes = typeline.split('-').collect_vec();
    let (types, subtypes) = match types_and_subtypes.as_slice() {
        [types] => (types, &""),
        [types, subtypes] => (types, subtypes),
        _ => return Err(anyhow!("Invalid typeline {}", typeline)),
    };

    let types = types
        .split(' ')
        .filter(|ty| !ty.is_empty())
        .map(|ty| Type::try_from(ty).with_context(|| format!("Parsing {}", ty)))
        .collect::<anyhow::Result<_>>()?;
    let subtypes = subtypes
        .split(' ')
        .filter(|ty| !ty.is_empty())
        .map(|ty| Subtype::try_from(ty).with_context(|| format!("Parsing {}", ty)))
        .collect::<anyhow::Result<_>>()?;

    Ok((types, subtypes))
}
