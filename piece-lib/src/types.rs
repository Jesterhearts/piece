use derive_more::{Deref, DerefMut};
use indexmap::IndexSet;

use crate::protogen::types::{Subtype, Type};

#[derive(Debug, Clone, Deref, DerefMut, PartialEq, Eq, Default)]
pub struct TypeSet(IndexSet<Type>);

impl From<&Vec<Type>> for TypeSet {
    fn from(values: &Vec<Type>) -> Self {
        Self(values.iter().copied().collect())
    }
}

impl From<&Vec<protobuf::EnumOrUnknown<Type>>> for TypeSet {
    fn from(values: &Vec<protobuf::EnumOrUnknown<Type>>) -> Self {
        Self(
            values
                .iter()
                .map(protobuf::EnumOrUnknown::enum_value)
                .map(Result::unwrap)
                .collect(),
        )
    }
}

impl From<&[Type]> for TypeSet {
    fn from(values: &[Type]) -> Self {
        Self(values.iter().copied().collect())
    }
}

impl<const C: usize> From<[Type; C]> for TypeSet {
    fn from(value: [Type; C]) -> Self {
        Self::from(value.as_slice())
    }
}

#[derive(Debug, Clone, Deref, DerefMut, PartialEq, Eq, Default)]
pub struct SubtypeSet(IndexSet<Subtype>);

impl From<&Vec<Subtype>> for SubtypeSet {
    fn from(values: &Vec<Subtype>) -> Self {
        Self(values.iter().copied().collect())
    }
}

impl From<&Vec<protobuf::EnumOrUnknown<Subtype>>> for SubtypeSet {
    fn from(values: &Vec<protobuf::EnumOrUnknown<Subtype>>) -> Self {
        Self(
            values
                .iter()
                .map(protobuf::EnumOrUnknown::enum_value)
                .map(Result::unwrap)
                .collect(),
        )
    }
}

impl From<&[Subtype]> for SubtypeSet {
    fn from(values: &[Subtype]) -> Self {
        Self(values.iter().copied().collect())
    }
}

impl<const C: usize> From<[Subtype; C]> for SubtypeSet {
    fn from(value: [Subtype; C]) -> Self {
        Self::from(value.as_slice())
    }
}

impl Subtype {
    pub(crate) fn is_creature_type(&self) -> bool {
        matches!(
            self,
            Subtype::ADVISOR
                | Subtype::AETHERBORN
                | Subtype::ALIEN
                | Subtype::ALLY
                | Subtype::ANGEL
                | Subtype::ANTELOPE
                | Subtype::APE
                | Subtype::ARCHER
                | Subtype::ARCHON
                | Subtype::ARMY
                | Subtype::ARTIFICER
                | Subtype::ASSASSIN
                | Subtype::ASSEMBLY_WORKER
                | Subtype::ASTARTES
                | Subtype::ATOG
                | Subtype::AUROCHS
                | Subtype::AVATAR
                | Subtype::AZRA
                | Subtype::BADGER
                | Subtype::BALLOON
                | Subtype::BARBARIAN
                | Subtype::BARD
                | Subtype::BASILISK
                | Subtype::BAT
                | Subtype::BEAR
                | Subtype::BEAST
                | Subtype::BEEBLE
                | Subtype::BEHOLDER
                | Subtype::BERSERKER
                | Subtype::BIRD
                | Subtype::BLINKMOTH
                | Subtype::BOAR
                | Subtype::BRINGER
                | Subtype::BRUSHWAGG
                | Subtype::CAMARID
                | Subtype::CAMEL
                | Subtype::CARIBOU
                | Subtype::CARRIER
                | Subtype::CAT
                | Subtype::CENTAUR
                | Subtype::CEPHALID
                | Subtype::CHILD
                | Subtype::CHIMERA
                | Subtype::CITIZEN
                | Subtype::CLERIC
                | Subtype::CLOWN
                | Subtype::COCKATRICE
                | Subtype::CONSTRUCT
                | Subtype::COWARD
                | Subtype::CRAB
                | Subtype::CROCODILE
                | Subtype::CTAN
                | Subtype::CUSTODES
                | Subtype::CYBERMAN
                | Subtype::CYCLOPS
                | Subtype::DALEK
                | Subtype::DAUTHI
                | Subtype::DEMIGOD
                | Subtype::DEMON
                | Subtype::DESERTER
                | Subtype::DETECTIVE
                | Subtype::DEVIL
                | Subtype::DINOSAUR
                | Subtype::DJINN
                | Subtype::DOCTOR
                | Subtype::DOG
                | Subtype::DRAGON
                | Subtype::DRAKE
                | Subtype::DREADNOUGHT
                | Subtype::DRONE
                | Subtype::DRUID
                | Subtype::DRYAD
                | Subtype::DWARF
                | Subtype::EFREET
                | Subtype::EGG
                | Subtype::ELDER
                | Subtype::ELDRAZI
                | Subtype::ELEMENTAL
                | Subtype::ELEPHANT
                | Subtype::ELF
                | Subtype::ELK
                | Subtype::EMPLOYEE
                | Subtype::EYE
                | Subtype::FAERIE
                | Subtype::FERRET
                | Subtype::FISH
                | Subtype::FLAGBEARER
                | Subtype::FOX
                | Subtype::FRACTAL
                | Subtype::FROG
                | Subtype::FUNGUS
                | Subtype::GAMER
                | Subtype::GARGOYLE
                | Subtype::GERM
                | Subtype::GIANT
                | Subtype::GITH
                | Subtype::GNOLL
                | Subtype::GNOME
                | Subtype::GOAT
                | Subtype::GOBLIN
                | Subtype::GOD
                | Subtype::GOLEM
                | Subtype::GORGON
                | Subtype::GRAVEBORN
                | Subtype::GREMLIN
                | Subtype::GRIFFIN
                | Subtype::GUEST
                | Subtype::HAG
                | Subtype::HALFLING
                | Subtype::HAMSTER
                | Subtype::HARPY
                | Subtype::HELLION
                | Subtype::HIPPO
                | Subtype::HIPPOGRIFF
                | Subtype::HOMARID
                | Subtype::HOMUNCULUS
                | Subtype::HORROR
                | Subtype::HORSE
                | Subtype::HUMAN
                | Subtype::HYDRA
                | Subtype::HYENA
                | Subtype::ILLUSION
                | Subtype::IMP
                | Subtype::INCARNATION
                | Subtype::INKLING
                | Subtype::INQUISITOR
                | Subtype::INSECT
                | Subtype::JACKAL
                | Subtype::JELLYFISH
                | Subtype::JUGGERNAUT
                | Subtype::KAVU
                | Subtype::KIRIN
                | Subtype::KITHKIN
                | Subtype::KNIGHT
                | Subtype::KOBOLD
                | Subtype::KOR
                | Subtype::KRAKEN
                | Subtype::LAMIA
                | Subtype::LAMMASU
                | Subtype::LEECH
                | Subtype::LEVIATHAN
                | Subtype::LHURGOYF
                | Subtype::LICID
                | Subtype::LIZARD
                | Subtype::LORD
                | Subtype::MANTICORE
                | Subtype::MASTICORE
                | Subtype::MERCENARY
                | Subtype::MERFOLK
                | Subtype::METATHRAN
                | Subtype::MINION
                | Subtype::MINOTAUR
                | Subtype::MITE
                | Subtype::MOLE
                | Subtype::MONGER
                | Subtype::MONGOOSE
                | Subtype::MONK
                | Subtype::MONKEY
                | Subtype::MOONFOLK
                | Subtype::MOUSE
                | Subtype::MUTANT
                | Subtype::MYR
                | Subtype::MYSTIC
                | Subtype::NAGA
                | Subtype::NAUTILUS
                | Subtype::NECRON
                | Subtype::NEPHILIM
                | Subtype::NIGHTMARE
                | Subtype::NIGHTSTALKER
                | Subtype::NINJA
                | Subtype::NOBLE
                | Subtype::NOGGLE
                | Subtype::NOMAD
                | Subtype::NYMPH
                | Subtype::OCTOPUS
                | Subtype::OGRE
                | Subtype::OOZE
                | Subtype::ORB
                | Subtype::ORC
                | Subtype::ORGG
                | Subtype::OTTER
                | Subtype::OUPHE
                | Subtype::OX
                | Subtype::OYSTER
                | Subtype::PANGOLIN
                | Subtype::PEASANT
                | Subtype::PEGASUS
                | Subtype::PENTAVITE
                | Subtype::PERFORMER
                | Subtype::PEST
                | Subtype::PHELDDAGRIF
                | Subtype::PHOENIX
                | Subtype::PHYREXIAN
                | Subtype::PILOT
                | Subtype::PINCHER
                | Subtype::PIRATE
                | Subtype::PLANT
                | Subtype::PRAETOR
                | Subtype::PRIMARCH
                | Subtype::PRISM
                | Subtype::PROCESSOR
                | Subtype::RACCOON
                | Subtype::RABBIT
                | Subtype::RANGER
                | Subtype::RAT
                | Subtype::REBEL
                | Subtype::REFLECTION
                | Subtype::RHINO
                | Subtype::RIGGER
                | Subtype::ROBOT
                | Subtype::ROGUE
                | Subtype::SABLE
                | Subtype::SALAMANDER
                | Subtype::SAMURAI
                | Subtype::SAND
                | Subtype::SAPROLING
                | Subtype::SATYR
                | Subtype::SCARECROW
                | Subtype::SCIENTIST
                | Subtype::SCION
                | Subtype::SCORPION
                | Subtype::SCOUT
                | Subtype::SCULPTURE
                | Subtype::SERF
                | Subtype::SERPENT
                | Subtype::SERVO
                | Subtype::SHADE
                | Subtype::SHAMAN
                | Subtype::SHAPESHIFTER
                | Subtype::SHARK
                | Subtype::SHEEP
                | Subtype::SIREN
                | Subtype::SKELETON
                | Subtype::SLITH
                | Subtype::SLIVER
                | Subtype::SLUG
                | Subtype::SNAKE
                | Subtype::SOLDIER
                | Subtype::SOLTARI
                | Subtype::SPAWN
                | Subtype::SPECTER
                | Subtype::SPELLSHAPER
                | Subtype::SPHINX
                | Subtype::SPIDER
                | Subtype::SPIKE
                | Subtype::SPIRIT
                | Subtype::SPLINTER
                | Subtype::SPONGE
                | Subtype::SQUID
                | Subtype::SQUIRREL
                | Subtype::STARFISH
                | Subtype::SURRAKAR
                | Subtype::SURVIVOR
                | Subtype::TENTACLE
                | Subtype::TETRAVITE
                | Subtype::THALAKOS
                | Subtype::THOPTER
                | Subtype::THRULL
                | Subtype::TIEFLING
                | Subtype::TIME
                | Subtype::TREEFOLK
                | Subtype::TRILOBITE
                | Subtype::TRISKELAVITE
                | Subtype::TROLL
                | Subtype::TURTLE
                | Subtype::TYRANID
                | Subtype::UNICORN
                | Subtype::VAMPIRE
                | Subtype::VEDALKEN
                | Subtype::VIASHINO
                | Subtype::VOLVER
                | Subtype::WALL
                | Subtype::WALRUS
                | Subtype::WARLOCK
                | Subtype::WARRIOR
                | Subtype::WEIRD
                | Subtype::WEREWOLF
                | Subtype::WHALE
                | Subtype::WIZARD
                | Subtype::WOLF
                | Subtype::WOLVERINE
                | Subtype::WOMBAT
                | Subtype::WORM
                | Subtype::WRAITH
                | Subtype::WURM
                | Subtype::YETI
                | Subtype::ZOMBIE
                | Subtype::ZUBERA
        )
    }
}
