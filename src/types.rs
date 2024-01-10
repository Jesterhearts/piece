use std::{collections::HashSet, str::FromStr};

use derive_more::{Deref, DerefMut};

use crate::protogen::types::{subtype, type_, Subtype, Type};

#[derive(Debug, Clone, Deref, DerefMut, PartialEq, Eq, Default)]
pub struct TypeSet(HashSet<String>);

impl From<&Vec<Type>> for TypeSet {
    fn from(values: &Vec<Type>) -> Self {
        let mut set = HashSet::with_capacity(values.len());

        for value in values.iter().cloned() {
            set.insert(value.type_.as_ref().unwrap().as_ref().to_string());
        }

        Self(set)
    }
}

impl From<&[type_::Type]> for TypeSet {
    fn from(values: &[type_::Type]) -> Self {
        let mut set = HashSet::with_capacity(values.len());

        for value in values.iter() {
            set.insert(value.as_ref().to_string());
        }

        Self(set)
    }
}

impl<const C: usize> From<[type_::Type; C]> for TypeSet {
    fn from(value: [type_::Type; C]) -> Self {
        Self::from(value.as_slice())
    }
}

impl From<&[type_::TypeDiscriminants]> for TypeSet {
    fn from(values: &[type_::TypeDiscriminants]) -> Self {
        let mut set = HashSet::with_capacity(values.len());

        for value in values.iter() {
            set.insert(value.as_ref().to_string());
        }

        Self(set)
    }
}

impl<const C: usize> From<[type_::TypeDiscriminants; C]> for TypeSet {
    fn from(value: [type_::TypeDiscriminants; C]) -> Self {
        Self::from(value.as_slice())
    }
}

#[derive(Debug, Clone, Deref, DerefMut, PartialEq, Eq, Default)]
pub struct SubtypeSet(HashSet<String>);

impl From<&Vec<Subtype>> for SubtypeSet {
    fn from(values: &Vec<Subtype>) -> Self {
        let mut set = HashSet::with_capacity(values.len());

        for value in values.iter() {
            set.insert(value.subtype.as_ref().unwrap().as_ref().to_string());
        }

        Self(set)
    }
}

impl From<&[subtype::Subtype]> for SubtypeSet {
    fn from(values: &[subtype::Subtype]) -> Self {
        let mut set = HashSet::with_capacity(values.len());

        for value in values.iter() {
            set.insert(value.as_ref().to_string());
        }

        Self(set)
    }
}

impl<const C: usize> From<[subtype::Subtype; C]> for SubtypeSet {
    fn from(value: [subtype::Subtype; C]) -> Self {
        Self::from(value.as_slice())
    }
}

impl Subtype {
    pub(crate) fn is_creature_type(&self) -> bool {
        matches!(
            self.subtype.as_ref().unwrap(),
            subtype::Subtype::Advisor(_)
                | subtype::Subtype::Aetherborn(_)
                | subtype::Subtype::Alien(_)
                | subtype::Subtype::Ally(_)
                | subtype::Subtype::Angel(_)
                | subtype::Subtype::Antelope(_)
                | subtype::Subtype::Ape(_)
                | subtype::Subtype::Archer(_)
                | subtype::Subtype::Archon(_)
                | subtype::Subtype::Army(_)
                | subtype::Subtype::Artificer(_)
                | subtype::Subtype::Assassin(_)
                | subtype::Subtype::AssemblyWorker(_)
                | subtype::Subtype::Astartes(_)
                | subtype::Subtype::Atog(_)
                | subtype::Subtype::Aurochs(_)
                | subtype::Subtype::Avatar(_)
                | subtype::Subtype::Azra(_)
                | subtype::Subtype::Badger(_)
                | subtype::Subtype::Balloon(_)
                | subtype::Subtype::Barbarian(_)
                | subtype::Subtype::Bard(_)
                | subtype::Subtype::Basilisk(_)
                | subtype::Subtype::Bat(_)
                | subtype::Subtype::Bear(_)
                | subtype::Subtype::Beast(_)
                | subtype::Subtype::Beeble(_)
                | subtype::Subtype::Beholder(_)
                | subtype::Subtype::Berserker(_)
                | subtype::Subtype::Bird(_)
                | subtype::Subtype::Blinkmoth(_)
                | subtype::Subtype::Boar(_)
                | subtype::Subtype::Bringer(_)
                | subtype::Subtype::Brushwagg(_)
                | subtype::Subtype::Camarid(_)
                | subtype::Subtype::Camel(_)
                | subtype::Subtype::Caribou(_)
                | subtype::Subtype::Carrier(_)
                | subtype::Subtype::Cat(_)
                | subtype::Subtype::Centaur(_)
                | subtype::Subtype::Cephalid(_)
                | subtype::Subtype::Child(_)
                | subtype::Subtype::Chimera(_)
                | subtype::Subtype::Citizen(_)
                | subtype::Subtype::Cleric(_)
                | subtype::Subtype::Clown(_)
                | subtype::Subtype::Cockatrice(_)
                | subtype::Subtype::Construct(_)
                | subtype::Subtype::Coward(_)
                | subtype::Subtype::Crab(_)
                | subtype::Subtype::Crocodile(_)
                | subtype::Subtype::Ctan(_)
                | subtype::Subtype::Custodes(_)
                | subtype::Subtype::Cyberman(_)
                | subtype::Subtype::Cyclops(_)
                | subtype::Subtype::Dalek(_)
                | subtype::Subtype::Dauthi(_)
                | subtype::Subtype::Demigod(_)
                | subtype::Subtype::Demon(_)
                | subtype::Subtype::Deserter(_)
                | subtype::Subtype::Detective(_)
                | subtype::Subtype::Devil(_)
                | subtype::Subtype::Dinosaur(_)
                | subtype::Subtype::Djinn(_)
                | subtype::Subtype::Doctor(_)
                | subtype::Subtype::Dog(_)
                | subtype::Subtype::Dragon(_)
                | subtype::Subtype::Drake(_)
                | subtype::Subtype::Dreadnought(_)
                | subtype::Subtype::Drone(_)
                | subtype::Subtype::Druid(_)
                | subtype::Subtype::Dryad(_)
                | subtype::Subtype::Dwarf(_)
                | subtype::Subtype::Efreet(_)
                | subtype::Subtype::Egg(_)
                | subtype::Subtype::Elder(_)
                | subtype::Subtype::Eldrazi(_)
                | subtype::Subtype::Elemental(_)
                | subtype::Subtype::Elephant(_)
                | subtype::Subtype::Elf(_)
                | subtype::Subtype::Elk(_)
                | subtype::Subtype::Employee(_)
                | subtype::Subtype::Eye(_)
                | subtype::Subtype::Faerie(_)
                | subtype::Subtype::Ferret(_)
                | subtype::Subtype::Fish(_)
                | subtype::Subtype::Flagbearer(_)
                | subtype::Subtype::Fox(_)
                | subtype::Subtype::Fractal(_)
                | subtype::Subtype::Frog(_)
                | subtype::Subtype::Fungus(_)
                | subtype::Subtype::Gamer(_)
                | subtype::Subtype::Gargoyle(_)
                | subtype::Subtype::Germ(_)
                | subtype::Subtype::Giant(_)
                | subtype::Subtype::Gith(_)
                | subtype::Subtype::Gnoll(_)
                | subtype::Subtype::Gnome(_)
                | subtype::Subtype::Goat(_)
                | subtype::Subtype::Goblin(_)
                | subtype::Subtype::God(_)
                | subtype::Subtype::Golem(_)
                | subtype::Subtype::Gorgon(_)
                | subtype::Subtype::Graveborn(_)
                | subtype::Subtype::Gremlin(_)
                | subtype::Subtype::Griffin(_)
                | subtype::Subtype::Guest(_)
                | subtype::Subtype::Hag(_)
                | subtype::Subtype::Halfling(_)
                | subtype::Subtype::Hamster(_)
                | subtype::Subtype::Harpy(_)
                | subtype::Subtype::Hellion(_)
                | subtype::Subtype::Hippo(_)
                | subtype::Subtype::Hippogriff(_)
                | subtype::Subtype::Homarid(_)
                | subtype::Subtype::Homunculus(_)
                | subtype::Subtype::Horror(_)
                | subtype::Subtype::Horse(_)
                | subtype::Subtype::Human(_)
                | subtype::Subtype::Hydra(_)
                | subtype::Subtype::Hyena(_)
                | subtype::Subtype::Illusion(_)
                | subtype::Subtype::Imp(_)
                | subtype::Subtype::Incarnation(_)
                | subtype::Subtype::Inkling(_)
                | subtype::Subtype::Inquisitor(_)
                | subtype::Subtype::Insect(_)
                | subtype::Subtype::Jackal(_)
                | subtype::Subtype::Jellyfish(_)
                | subtype::Subtype::Juggernaut(_)
                | subtype::Subtype::Kavu(_)
                | subtype::Subtype::Kirin(_)
                | subtype::Subtype::Kithkin(_)
                | subtype::Subtype::Knight(_)
                | subtype::Subtype::Kobold(_)
                | subtype::Subtype::Kor(_)
                | subtype::Subtype::Kraken(_)
                | subtype::Subtype::Lamia(_)
                | subtype::Subtype::Lammasu(_)
                | subtype::Subtype::Leech(_)
                | subtype::Subtype::Leviathan(_)
                | subtype::Subtype::Lhurgoyf(_)
                | subtype::Subtype::Licid(_)
                | subtype::Subtype::Lizard(_)
                | subtype::Subtype::Lord(_)
                | subtype::Subtype::Manticore(_)
                | subtype::Subtype::Masticore(_)
                | subtype::Subtype::Mercenary(_)
                | subtype::Subtype::Merfolk(_)
                | subtype::Subtype::Metathran(_)
                | subtype::Subtype::Minion(_)
                | subtype::Subtype::Minotaur(_)
                | subtype::Subtype::Mite(_)
                | subtype::Subtype::Mole(_)
                | subtype::Subtype::Monger(_)
                | subtype::Subtype::Mongoose(_)
                | subtype::Subtype::Monk(_)
                | subtype::Subtype::Monkey(_)
                | subtype::Subtype::Moonfolk(_)
                | subtype::Subtype::Mouse(_)
                | subtype::Subtype::Mutant(_)
                | subtype::Subtype::Myr(_)
                | subtype::Subtype::Mystic(_)
                | subtype::Subtype::Naga(_)
                | subtype::Subtype::Nautilus(_)
                | subtype::Subtype::Necron(_)
                | subtype::Subtype::Nephilim(_)
                | subtype::Subtype::Nightmare(_)
                | subtype::Subtype::Nightstalker(_)
                | subtype::Subtype::Ninja(_)
                | subtype::Subtype::Noble(_)
                | subtype::Subtype::Noggle(_)
                | subtype::Subtype::Nomad(_)
                | subtype::Subtype::Nymph(_)
                | subtype::Subtype::Octopus(_)
                | subtype::Subtype::Ogre(_)
                | subtype::Subtype::Ooze(_)
                | subtype::Subtype::Orb(_)
                | subtype::Subtype::Orc(_)
                | subtype::Subtype::Orgg(_)
                | subtype::Subtype::Otter(_)
                | subtype::Subtype::Ouphe(_)
                | subtype::Subtype::Ox(_)
                | subtype::Subtype::Oyster(_)
                | subtype::Subtype::Pangolin(_)
                | subtype::Subtype::Peasant(_)
                | subtype::Subtype::Pegasus(_)
                | subtype::Subtype::Pentavite(_)
                | subtype::Subtype::Performer(_)
                | subtype::Subtype::Pest(_)
                | subtype::Subtype::Phelddagrif(_)
                | subtype::Subtype::Phoenix(_)
                | subtype::Subtype::Phyrexian(_)
                | subtype::Subtype::Pilot(_)
                | subtype::Subtype::Pincher(_)
                | subtype::Subtype::Pirate(_)
                | subtype::Subtype::Plant(_)
                | subtype::Subtype::Praetor(_)
                | subtype::Subtype::Primarch(_)
                | subtype::Subtype::Prism(_)
                | subtype::Subtype::Processor(_)
                | subtype::Subtype::Raccoon(_)
                | subtype::Subtype::Rabbit(_)
                | subtype::Subtype::Ranger(_)
                | subtype::Subtype::Rat(_)
                | subtype::Subtype::Rebel(_)
                | subtype::Subtype::Reflection(_)
                | subtype::Subtype::Rhino(_)
                | subtype::Subtype::Rigger(_)
                | subtype::Subtype::Robot(_)
                | subtype::Subtype::Rogue(_)
                | subtype::Subtype::Sable(_)
                | subtype::Subtype::Salamander(_)
                | subtype::Subtype::Samurai(_)
                | subtype::Subtype::Sand(_)
                | subtype::Subtype::Saproling(_)
                | subtype::Subtype::Satyr(_)
                | subtype::Subtype::Scarecrow(_)
                | subtype::Subtype::Scientist(_)
                | subtype::Subtype::Scion(_)
                | subtype::Subtype::Scorpion(_)
                | subtype::Subtype::Scout(_)
                | subtype::Subtype::Sculpture(_)
                | subtype::Subtype::Serf(_)
                | subtype::Subtype::Serpent(_)
                | subtype::Subtype::Servo(_)
                | subtype::Subtype::Shade(_)
                | subtype::Subtype::Shaman(_)
                | subtype::Subtype::Shapeshifter(_)
                | subtype::Subtype::Shark(_)
                | subtype::Subtype::Sheep(_)
                | subtype::Subtype::Siren(_)
                | subtype::Subtype::Skeleton(_)
                | subtype::Subtype::Slith(_)
                | subtype::Subtype::Sliver(_)
                | subtype::Subtype::Slug(_)
                | subtype::Subtype::Snake(_)
                | subtype::Subtype::Soldier(_)
                | subtype::Subtype::Soltari(_)
                | subtype::Subtype::Spawn(_)
                | subtype::Subtype::Specter(_)
                | subtype::Subtype::Spellshaper(_)
                | subtype::Subtype::Sphinx(_)
                | subtype::Subtype::Spider(_)
                | subtype::Subtype::Spike(_)
                | subtype::Subtype::Spirit(_)
                | subtype::Subtype::Splinter(_)
                | subtype::Subtype::Sponge(_)
                | subtype::Subtype::Squid(_)
                | subtype::Subtype::Squirrel(_)
                | subtype::Subtype::Starfish(_)
                | subtype::Subtype::Surrakar(_)
                | subtype::Subtype::Survivor(_)
                | subtype::Subtype::Tentacle(_)
                | subtype::Subtype::Tetravite(_)
                | subtype::Subtype::Thalakos(_)
                | subtype::Subtype::Thopter(_)
                | subtype::Subtype::Thrull(_)
                | subtype::Subtype::Tiefling(_)
                | subtype::Subtype::Time(_)
                | subtype::Subtype::Treefolk(_)
                | subtype::Subtype::Trilobite(_)
                | subtype::Subtype::Triskelavite(_)
                | subtype::Subtype::Troll(_)
                | subtype::Subtype::Turtle(_)
                | subtype::Subtype::Tyranid(_)
                | subtype::Subtype::Unicorn(_)
                | subtype::Subtype::Vampire(_)
                | subtype::Subtype::Vedalken(_)
                | subtype::Subtype::Viashino(_)
                | subtype::Subtype::Volver(_)
                | subtype::Subtype::Wall(_)
                | subtype::Subtype::Walrus(_)
                | subtype::Subtype::Warlock(_)
                | subtype::Subtype::Warrior(_)
                | subtype::Subtype::Weird(_)
                | subtype::Subtype::Werewolf(_)
                | subtype::Subtype::Whale(_)
                | subtype::Subtype::Wizard(_)
                | subtype::Subtype::Wolf(_)
                | subtype::Subtype::Wolverine(_)
                | subtype::Subtype::Wombat(_)
                | subtype::Subtype::Worm(_)
                | subtype::Subtype::Wraith(_)
                | subtype::Subtype::Wurm(_)
                | subtype::Subtype::Yeti(_)
                | subtype::Subtype::Zombie(_)
                | subtype::Subtype::Zubera(_)
        )
    }
}

impl FromStr for Type {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let type_ = type_::Type::from_str(s)?;
        Ok(Self {
            type_: Some(type_),
            ..Default::default()
        })
    }
}

impl FromStr for Subtype {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let subtype = subtype::Subtype::from_str(s)?;
        Ok(Self {
            subtype: Some(subtype),
            ..Default::default()
        })
    }
}

impl AsRef<str> for Type {
    fn as_ref(&self) -> &str {
        self.type_.as_ref().unwrap().as_ref()
    }
}

impl AsRef<str> for Subtype {
    fn as_ref(&self) -> &str {
        self.subtype.as_ref().unwrap().as_ref()
    }
}
