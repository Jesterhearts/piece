mod add_counters;
mod apply_modifier;
mod apply_to_each_target;
mod attack_selected;
mod ban_attacking_this_turn;
mod cascade;
mod cast_selected;
mod choose_attackers;
mod clear_selected;
mod clone_selected;
mod complete_spell_resolution;
mod copy_spell_or_ability;
mod counter_spell;
mod create_token;
mod create_token_clone_of_selected;
mod cycling;
mod damage_selected;
mod declare_attacking;
mod destroy_selected;
mod discard;
mod discard_selected;
mod discover;
mod draw_cards;
mod equip;
mod exile_graveyard;
mod explore;
mod for_each_mana_of_source;
mod gain_life;
mod gain_mana;
mod if_then_else;
mod lose_life;
mod manifest;
mod mill;
mod modal;
mod move_to_battlefield;
mod move_to_bottom_of_library;
mod move_to_exile;
mod move_to_graveyard;
mod move_to_hand;
mod move_to_stack;
mod move_to_top_of_library;
mod multiply_tokens;
mod nothing;
mod ovewrite;
mod pay_costs;
mod player_loses;
mod pop_selected;
mod push_selected;
mod remove_counters;
mod reorder_selected;
mod reveal;
mod sacrifice;
mod scry;
mod select_all;
mod select_all_players;
mod select_destinations;
mod select_effect_controller;
mod select_for_each_player;
mod select_mode;
mod select_non_targeting;
mod select_source;
mod select_target_controller;
mod select_targets;
mod select_top_of_library;
mod spend_mana;
mod tap;
mod transform;
mod tutor_library;
mod unless;
mod untap;

use std::{collections::VecDeque, fmt::Debug, vec};

use derive_more::{Deref, DerefMut};
use itertools::Itertools;

use crate::{
    in_play::{CardId, Database},
    log::LogId,
    player::Owner,
    protogen::{
        cost::XIs,
        effects::{
            count, effect, replacement_effect::Replacing, target_selection::Selector,
            tutor_library::target::Destination, Count, Effect, ReorderSelected, TargetSelection,
        },
        targets::{Location, Restriction},
        triggers,
    },
    stack::{Selected, TargetType},
};

impl PartialEq<triggers::Location> for Location {
    fn eq(&self, other: &triggers::Location) -> bool {
        match self {
            Location::ON_BATTLEFIELD => matches!(
                other,
                triggers::Location::ANYWHERE | triggers::Location::BATTLEFIELD
            ),
            Location::IN_HAND => matches!(
                other,
                triggers::Location::ANYWHERE | triggers::Location::HAND
            ),
            Location::IN_LIBRARY => matches!(
                other,
                triggers::Location::ANYWHERE | triggers::Location::LIBRARY
            ),
            Location::IN_GRAVEYARD => matches!(other, triggers::Location::ANYWHERE),
            Location::IN_EXILE => matches!(other, triggers::Location::ANYWHERE),
            Location::IN_STACK => matches!(other, triggers::Location::ANYWHERE),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub enum SelectionResult {
    TryAgain,
    Complete,
    PendingChoice,
}

#[derive(Debug)]
pub enum ApplyResult {
    PushFront(EffectBundle),
    PushBack(EffectBundle),
}

#[derive(Debug)]
pub enum Options {
    MandatoryList(Vec<(usize, String)>),
    OptionalList(Vec<(usize, String)>),
    ListWithDefault(Vec<(usize, String)>),
}

impl Options {
    pub fn is_empty(&self) -> bool {
        match self {
            Options::MandatoryList(opts)
            | Options::OptionalList(opts)
            | Options::ListWithDefault(opts) => opts.is_empty(),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Options::MandatoryList(opts)
            | Options::OptionalList(opts)
            | Options::ListWithDefault(opts) => opts.len(),
        }
    }
}

#[enum_delegate::implement_for(crate::protogen::effects::effect::Effect,
    enum Effect {
        AddCounters(AddCounters),
        ApplyModifier(ApplyModifier),
        ApplyToEachTarget(ApplyToEachTarget),
        AttackSelected(AttackSelected),
        BanAttackingThisTurn(BanAttackingThisTurn),
        Cascade(Cascade),
        CastSelected(CastSelected),
        ChooseAttackers(ChooseAttackers),
        ClearSelected(ClearSelected),
        CloneSelected(CloneSelected),
        CompleteSpellResolution(CompleteSpellResolution),
        CopySpellOrAbility(CopySpellOrAbility),
        CounterSpell(CounterSpell),
        CreateToken(CreateToken),
        CreateTokenCloneOfSelected(CreateTokenCloneOfSelected),
        Cycling(Cycling),
        DamageSelected(DamageSelected),
        DeclareAttacking(DeclareAttacking),
        DestroySelected(DestroySelected),
        Discard(Discard),
        DiscardSelected(DiscardSelected),
        Discover(Discover),
        DrawCards(DrawCards),
        Equip(Equip),
        ExileGraveyard(ExileGraveyard),
        Explore(Explore),
        ForEachManaOfSource(ForEachManaOfSource),
        GainLife(GainLife),
        GainMana(GainMana),
        IfThenElse(IfThenElse),
        LoseLife(LoseLife),
        Manifest(Manifest),
        Mill(Mill),
        Modal(Modal),
        MoveToBattlefield(MoveToBattlefield),
        MoveToBottomOfLibrary(MoveToBottomOfLibrary),
        MoveToExile(MoveToExile),
        MoveToGraveyard(MoveToGraveyard),
        MoveToHand(MoveToHand),
        MoveToStack(MoveToStack),
        MoveToTopOfLibrary(MoveToTopOfLibrary),
        MultiplyTokens(MultiplyTokens),
        Nothing(Nothing),
        Overwrite(Overwrite),
        PayCosts(PayCosts),
        PlayerLoses(PlayerLoses),
        PopSelected(PopSelected),
        PushSelected(PushSelected),
        RemoveCounters(RemoveCounters),
        ReorderSelected(ReorderSelected),
        Reveal(Reveal),
        Sacrifice(Sacrifice),
        Scry(Scry),
        SelectAll(SelectAll),
        SelectAllPlayers(SelectAllPlayers),
        SelectDestinations(SelectDestinations),
        SelectForEachPlayer(SelectForEachPlayer),
        SelectMode(SelectMode),
        SelectNonTargeting(SelectNonTargeting),
        SelectSource(SelectSource),
        SelectEffectController(SelectEffectController),
        SelectTargetController(SelectTargetController),
        SelectTargets(SelectTargets),
        SelectTopOfLibrary(SelectTopOfLibrary),
        SpendMana(SpendMana),
        Tap(Tap),
        Transform(Transform),
        TutorLibrary(TutorLibrary),
        Unless(Unless),
        Untap(Untap),
    }
)]
#[enum_delegate::implement_for(crate::protogen::effects::dest::Destination,
    enum Destination {
        MoveToBattlefield(MoveToBattlefield),
        MoveToBottomOfLibrary(MoveToBottomOfLibrary),
        MoveToExile(MoveToExile),
        MoveToGraveyard(MoveToGraveyard),
        MoveToHand(MoveToHand),
        MoveToTopOfLibrary(MoveToTopOfLibrary),
    }
)]
#[enum_delegate::implement_for(crate::protogen::effects::pay_cost::Cost,
    enum Cost {
        ExileCardsSharingType(ExileCardsSharingType),
        ExilePermanents(ExilePermanents),
        ExilePermanentsCmcX(ExilePermanentsCmcX),
        PayLife(PayLife),
        PayMana(PayMana),
        SacrificePermanent(SacrificePermanent),
        TapPermanent(TapPermanent),
        TapPermanentsPowerXOrMore(TapPermanentsPowerXOrMore),
    }
)]
pub(crate) trait EffectBehaviors {
    /// Which player has priority for this action.
    fn priority(
        &self,
        db: &Database,
        source: Option<CardId>,
        already_selected: &[Selected],
        modes: &[usize],
    ) -> Owner {
        let _ = already_selected;
        let _ = modes;

        if let Some(source) = source {
            db[source].controller.into()
        } else {
            db.turn.priority_player()
        }
    }

    fn description(
        &self,
        db: &Database,
        source: Option<CardId>,
        already_selected: &[Selected],
        modes: &[usize],
    ) -> String {
        let _ = db;
        let _ = source;
        let _ = already_selected;
        let _ = modes;

        String::default()
    }

    fn wants_input(
        &self,
        db: &Database,
        source: Option<CardId>,
        already_selected: &[Selected],
        modes: &[usize],
    ) -> bool {
        let _ = db;
        let _ = source;
        let _ = already_selected;
        let _ = modes;
        false
    }

    /// A textual list of choices represented by this effect. E.g. For SelectTargets, this will be the text list of targets which can be selected.
    fn options(
        &self,
        db: &Database,
        source: Option<CardId>,
        already_selected: &[Selected],
        modes: &[usize],
    ) -> Options {
        let _ = db;
        let _ = source;
        let _ = already_selected;
        let _ = modes;
        Options::OptionalList(vec![])
    }

    fn target_for_option(
        &self,
        db: &Database,
        source: Option<CardId>,
        already_selected: &[Selected],
        option: usize,
    ) -> Option<Selected> {
        let _ = db;
        let _ = source;

        already_selected.get(option).cloned()
    }

    /// Select the nth option.
    fn select(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        option: Option<usize>,
        selected: &mut SelectedStack,
    ) -> SelectionResult {
        let _ = db;
        let _ = source;
        let _ = option;
        let _ = selected;
        SelectionResult::Complete
    }

    /// Apply the effect to the database.
    #[must_use]
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        skip_replacement: bool,
    ) -> Vec<ApplyResult>;

    /// Apply the replacement effects to the bundle.
    fn apply_replacement(&self, effect: Effect) -> Vec<Effect> {
        vec![effect]
    }
}

#[derive(Debug, Clone, Default, Deref, DerefMut)]
pub(crate) struct SelectedStack {
    #[deref]
    #[deref_mut]
    pub(crate) current: Vec<Selected>,
    pub(crate) stack: VecDeque<Vec<Selected>>,

    pub(crate) modes: Vec<usize>,

    pub(crate) crafting: bool,
}

impl SelectedStack {
    pub(crate) fn new(current: Vec<Selected>) -> Self {
        Self {
            current,
            ..Default::default()
        }
    }

    pub(crate) fn save(&mut self) {
        self.stack.push_back(self.current.clone())
    }

    #[must_use]
    pub(crate) fn restore(&mut self) -> Vec<Selected> {
        let mut popped = self.stack.pop_back().unwrap_or_default();
        std::mem::swap(&mut self.current, &mut popped);
        popped
    }
}

#[derive(Debug, Default)]
pub struct EffectBundle {
    pub(crate) push_on_enter: Option<Vec<Selected>>,
    pub(crate) source: Option<CardId>,

    pub(crate) skip_replacement: bool,
    pub(crate) effects: Vec<Effect>,
    pub(crate) resolving: usize,
}

#[derive(Default, Debug)]
#[must_use]
pub struct PendingEffects {
    pub(crate) selected: SelectedStack,
    bundles: VecDeque<EffectBundle>,
}

impl PendingEffects {
    pub(crate) fn new(selected: SelectedStack) -> Self {
        Self {
            selected,
            ..Default::default()
        }
    }

    pub fn organize_stack(db: &Database) -> PendingEffects {
        let selected = db
            .stack
            .entries
            .keys()
            .copied()
            .map(|entry| Selected {
                location: Some(Location::IN_STACK),
                target_type: TargetType::Stack(entry),
                targeted: false,
                restrictions: vec![],
            })
            .collect_vec();

        Self {
            selected: SelectedStack::new(selected),
            bundles: VecDeque::from([EffectBundle {
                effects: vec![Effect {
                    effect: Some(ReorderSelected::default().into()),
                    ..Default::default()
                }],
                ..Default::default()
            }]),
        }
    }

    pub fn push_back(&mut self, bundle: EffectBundle) {
        self.bundles.push_back(bundle);
    }

    pub(crate) fn push_front(&mut self, bundle: EffectBundle) {
        self.bundles.push_front(bundle);
    }

    pub fn apply_result(&mut self, result: ApplyResult) {
        match result {
            ApplyResult::PushFront(bundle) => {
                self.bundles.push_front(bundle);
            }
            ApplyResult::PushBack(bundle) => self.bundles.push_back(bundle),
        }
    }

    pub fn apply_results(&mut self, other: impl IntoIterator<Item = ApplyResult>) {
        for result in other.into_iter() {
            match result {
                ApplyResult::PushFront(bundle) => {
                    self.bundles.push_front(bundle);
                }
                ApplyResult::PushBack(bundle) => {
                    self.bundles.push_back(bundle);
                }
            }
        }
    }

    pub fn extend(&mut self, other: PendingEffects) {
        self.bundles.extend(other.bundles);
    }

    pub fn is_empty(&self) -> bool {
        self.bundles.is_empty()
    }

    pub fn target_for_option(&self, db: &Database, option: usize) -> Option<Selected> {
        self.bundles.front().and_then(|first| {
            first.effects[first.resolving]
                .effect
                .as_ref()
                .unwrap()
                .target_for_option(db, first.source, &self.selected, option)
        })
    }

    pub fn priority(&self, db: &Database) -> Owner {
        self.bundles
            .front()
            .and_then(|first| {
                first
                    .effects
                    .get(first.resolving)
                    .map(|effect| (first.source, effect))
            })
            .map(|(source, first)| {
                first.effect.as_ref().unwrap().priority(
                    db,
                    source,
                    &self.selected,
                    &self.selected.modes,
                )
            })
            .unwrap_or_else(|| db.turn.priority_player())
    }

    pub fn description(&self, db: &Database) -> String {
        self.bundles
            .front()
            .map(|first| {
                first.effects[first.resolving]
                    .effect
                    .as_ref()
                    .unwrap()
                    .description(db, first.source, &self.selected, &self.selected.modes)
            })
            .unwrap_or_default()
    }

    pub fn wants_input(&self, db: &Database) -> bool {
        self.bundles
            .front()
            .and_then(|front| {
                front
                    .effects
                    .get(front.resolving)
                    .and_then(|first| first.effect.as_ref())
                    .map(|first| (first, front.source))
            })
            .map(|(first, source)| {
                first.wants_input(db, source, &self.selected, &self.selected.modes)
            })
            .unwrap_or_default()
    }

    pub fn resolve(&mut self, db: &mut Database, option: Option<usize>) -> SelectionResult {
        let mut applied = false;
        if option.is_none() {
            loop {
                let Some(first) = self.bundles.front_mut() else {
                    break;
                };

                if first.resolving == 0 && first.push_on_enter.is_some() {
                    self.selected.save();
                    self.selected.clear();
                    self.selected.extend(first.push_on_enter.take().unwrap());
                }

                let first_len = first.effects.len();
                let Some(effect) = first.effects.get_mut(first.resolving) else {
                    self.bundles.pop_front();
                    continue;
                };

                let Some(effect) = effect.effect.as_mut() else {
                    first.resolving += 1;
                    continue;
                };

                if effect.wants_input(db, first.source, &self.selected, &self.selected.modes) {
                    break;
                }

                applied = true;
                let results =
                    effect.apply(db, first.source, &mut self.selected, first.skip_replacement);

                first.resolving += 1;
                if first.resolving == first_len {
                    self.bundles.pop_front();
                }

                self.apply_results(results);
            }

            if applied {
                if self.bundles.is_empty() {
                    return SelectionResult::Complete;
                } else {
                    return SelectionResult::TryAgain;
                }
            }
        }

        if let Some(first) = self.bundles.front_mut() {
            if first.resolving == 0 && first.push_on_enter.is_some() {
                self.selected.save();
                self.selected.clear();
                self.selected.extend(first.push_on_enter.take().unwrap());
            }

            let effect = first.effects[first.resolving].effect.as_mut().unwrap();

            match effect.select(db, first.source, option, &mut self.selected) {
                SelectionResult::Complete => {
                    let results =
                        effect.apply(db, first.source, &mut self.selected, first.skip_replacement);

                    first.resolving += 1;
                    if first.resolving == first.effects.len() {
                        let _ = self.bundles.pop_front().unwrap();
                    }

                    self.apply_results(results);

                    if self.bundles.is_empty() {
                        SelectionResult::Complete
                    } else {
                        SelectionResult::TryAgain
                    }
                }
                r => r,
            }
        } else {
            SelectionResult::Complete
        }
    }

    pub fn options(&self, db: &Database) -> Options {
        self.bundles
            .front()
            .and_then(|first| {
                first
                    .effects
                    .get(first.resolving)
                    .map(|effect| (first.source, effect))
            })
            .map(|(source, effect)| {
                effect.effect.as_ref().unwrap().options(
                    db,
                    source,
                    &self.selected,
                    &self.selected.modes,
                )
            })
            .unwrap_or_else(|| Options::OptionalList(vec![]))
    }
}

impl From<EffectBundle> for PendingEffects {
    fn from(value: EffectBundle) -> Self {
        Self {
            bundles: VecDeque::from([value]),
            ..Default::default()
        }
    }
}

impl Count {
    pub(crate) fn count(
        &self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &[Selected],
    ) -> i32 {
        match self.count.as_ref().unwrap() {
            count::Count::Fixed(count) => count.count,
            count::Count::LeftBattlefieldThisTurn(left) => {
                if let Some(first) = selected.first().and_then(|first| first.id(db)) {
                    CardId::left_battlefield_this_turn(db)
                        .filter(|card| {
                            card.passes_restrictions(
                                db,
                                LogId::current(db),
                                first,
                                &left.restrictions,
                            )
                        })
                        .count() as i32
                } else {
                    warn!("No card selected when determining number of counters to place. Did you forget to select targets?");
                    0
                }
            }
            count::Count::NumberOfCountersOnSelected(counters) => {
                if let Some(first) = selected.first() {
                    if let Some(card) = first.id(db) {
                        *db[card]
                            .counters
                            .entry(counters.type_.enum_value().unwrap())
                            .or_default() as i32
                    } else {
                        todo!("number of counters on players")
                    }
                } else {
                    warn!("No card selected when determining number of counters to place. Did you forget to select targets?");
                    0
                }
            }
            count::Count::NumberOfPermanentsMatching(matching) => db
                .cards
                .keys()
                .filter(|card| {
                    card.passes_restrictions(
                        db,
                        LogId::current(db),
                        source.unwrap(),
                        &matching.restrictions,
                    )
                })
                .count() as i32,
            count::Count::X(x) => match x.x_is.enum_value().unwrap() {
                XIs::MANA_VALUE_OF_SELECTED => db[selected.first().unwrap().id(db).unwrap()]
                    .modified_cost
                    .cmc() as i32,
            },
            count::Count::XCost(_) => db[source.unwrap()].x_is as i32,
        }
    }
}

fn handle_replacements<T: Into<effect::Effect>>(
    db: &Database,
    source: Option<CardId>,
    replacing: Replacing,
    effect: T,
    passes_restrictions: impl Fn(CardId, &[Restriction]) -> bool,
) -> Vec<ApplyResult> {
    let replacements = db.replacement_abilities_watching(replacing);
    let replacements = replacements
        .into_iter()
        .filter(|(card, replacing)| passes_restrictions(*card, &replacing.restrictions))
        .map(|(_, replacement)| TargetType::ReplacementAbility(replacement))
        .map(|target| Selected {
            location: None,
            target_type: target,
            targeted: false,
            restrictions: vec![],
        })
        .collect_vec();

    vec![ApplyResult::PushFront(EffectBundle {
        push_on_enter: Some(replacements),
        effects: vec![ReorderSelected {
            associated_effect: protobuf::MessageField::some(Effect {
                effect: Some(effect.into()),
                ..Default::default()
            }),
            ..Default::default()
        }
        .into()],
        source,
        ..Default::default()
    })]
}

impl From<TargetSelection> for effect::Effect {
    fn from(val: TargetSelection) -> Self {
        match val.selector.unwrap() {
            Selector::Modal(modal) => modal.into(),
            Selector::SelectTargets(targets) => targets.into(),
            Selector::SelectNonTargeting(targets) => targets.into(),
            Selector::SelectForEachPlayer(targets) => targets.into(),
        }
    }
}

impl From<Destination> for effect::Effect {
    fn from(value: Destination) -> Self {
        match value {
            Destination::MoveToBattlefield(dest) => dest.into(),
            Destination::MoveToExile(dest) => dest.into(),
            Destination::MoveToGraveyard(dest) => dest.into(),
            Destination::MoveToHand(dest) => dest.into(),
            Destination::MoveToBottomOfLibrary(dest) => dest.into(),
            Destination::MoveToTopOfLibrary(dest) => dest.into(),
        }
    }
}

impl<T: Into<effect::Effect>> From<T> for Effect {
    fn from(value: T) -> Self {
        Self {
            effect: Some(value.into()),
            ..Default::default()
        }
    }
}
