mod add_counters;
mod apply_modifier;
mod attack_selected;
mod ban_attacking_this_turn;
mod cascade;
mod cast_selected;
mod clone_selected;
mod create_token;
mod create_token_clone_of_selected;
mod damage_selected;
mod declare_attacking;
mod destroy_selected;
mod discover;
mod draw_cards;
#[allow(clippy::module_inception)]
mod effects;
mod equip;
mod explore;
mod for_each_mana_of_source;
mod gain_life;
mod gain_mana;
mod lose_life;
mod manifest;
mod mill;
mod move_to_battlefield;
mod move_to_bottom_of_library;
mod move_to_exile;
mod move_to_graveyard;
mod move_to_hand;
mod move_to_stack;
mod move_to_top_of_library;
mod multiply_tokens;
mod pay_costs;
mod player_loses;
mod remove_counters;
mod reorder_selected;
mod reveal;
mod scry;
mod select_all;
mod select_attackers;
mod select_destinations;
mod select_for_each_player;
mod select_mode;
mod select_non_targeting;
mod select_self;
mod select_self_controller;
mod select_target_controller;
mod select_targets;
mod select_top_of_library;
mod spend_mana;
mod tap;
mod transform;
mod untap;

use std::collections::VecDeque;

use derive_more::{Deref, DerefMut};
use itertools::Itertools;

use crate::{
    in_play::{CardId, Database},
    log::LogId,
    player::Owner,
    protogen::{
        effects::{count, effect, replacement_effect::Replacing, Count, Effect, ReorderSelected},
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
pub enum SelectionResult {
    TryAgain,
    Complete,
    PendingChoice,
}

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
        AttackSelected(AttackSelected),
        BanAttackingThisTurn(BanAttackingThisTurn),
        Cascade(Cascade),
        CastSelected(CastSelected),
        CloneSelected(CloneSelected),
        CreateToken(CreateToken),
        CreateTokenCloneOfSelected(CreateTokenCloneOfSelected),
        DamageSelected(DamageSelected),
        DeclareAttacking(DeclareAttacking),
        DestroySelected(DestroySelected),
        Discover(Discover),
        DrawCards(DrawCards),
        Effects(Effects),
        Equip(Equip),
        Explore(Explore),
        ForEachManaOfSource(ForEachManaOfSource),
        GainLife(GainLife),
        GainMana(GainMana),
        LoseLife(LoseLife),
        Manifest(Manifest),
        Mill(Mill),
        MoveToBattlefield(MoveToBattlefield),
        MoveToBottomOfLibrary(MoveToBottomOfLibrary),
        MoveToExile(MoveToExile),
        MoveToGraveyard(MoveToGraveyard),
        MoveToHand(MoveToHand),
        MoveToStack(MoveToStack),
        MoveToTopOfLibrary(MoveToTopOfLibrary),
        MultiplyTokens(MultiplyTokens),
        PayCosts(PayCosts),
        PlayerLoses(PlayerLoses),
        RemoveCounters(RemoveCounters),
        ReorderSelected(ReorderSelected),
        Reveal(Reveal),
        Scry(Scry),
        SelectAll(SelectAll),
        SelectAttackers(SelectAttackers),
        SelectDestinations(SelectDestinations),
        SelectForEachPlayer(SelectForEachPlayer),
        SelectMode(SelectMode),
        SelectNonTargeting(SelectNonTargeting),
        SelectSelf(SelectSelf),
        SelectSelfController(SelectSelfController),
        SelectTargetController(SelectTargetController),
        SelectTargets(SelectTargets),
        SelectTopOfLibrary(SelectTopOfLibrary),
        SpendMana(SpendMana),
        Tap(Tap),
        Transform(Transform),
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
        PayMana(PayMana),
        SacrificePermanent(SacrificePermanent),
        TapPermanent(TapPermanent),
        TapPermanentsPowerXOrMore(TapPermanentsPowerXOrMore),
    }
)]
pub(crate) trait EffectBehaviors {
    /// Which player has priority for this action.
    fn priority(&self, db: &Database, source: Option<CardId>) -> Owner {
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
        modes: &[usize],
        option: usize,
    ) -> Option<Selected> {
        let _ = db;
        let _ = source;
        let _ = modes;

        already_selected.get(option).cloned()
    }

    /// Select the nth option.
    fn select(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        option: Option<usize>,
        selected: &mut SelectedStack,
        modes: &mut Vec<usize>,
    ) -> SelectionResult {
        let _ = db;
        let _ = source;
        let _ = option;
        let _ = selected;
        let _ = modes;
        SelectionResult::Complete
    }

    /// Apply the effect to the database.
    fn apply(
        &mut self,
        db: &mut Database,
        pending: &mut PendingEffects,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        modes: &[usize],
        skip_replacement: bool,
    );

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
    pub(crate) stack: Vec<Vec<Selected>>,
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
        self.stack.push(self.current.clone())
    }

    #[must_use]
    pub(crate) fn restore(&mut self) -> Vec<Selected> {
        let mut popped = self.stack.pop().unwrap_or_default();
        std::mem::swap(&mut self.current, &mut popped);
        popped
    }
}

#[derive(Debug, Default)]
pub struct EffectBundle {
    pub(crate) selected: SelectedStack,
    pub(crate) modes: Vec<usize>,
    pub(crate) source: Option<CardId>,
    pub(crate) effects: Vec<Effect>,
}

impl EffectBundle {
    pub fn organize_stack(db: &Database) -> Self {
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
            effects: vec![Effect {
                effect: Some(ReorderSelected::default().into()),
                ..Default::default()
            }],
            ..Default::default()
        }
    }
}

#[derive(Debug, Default)]
#[must_use]
pub struct PendingEffects {
    bundles: VecDeque<EffectBundle>,
    resolving: usize,
}

impl PendingEffects {
    pub fn push_back(&mut self, bundle: EffectBundle) {
        self.bundles.push_back(bundle);
    }

    pub(crate) fn push_front(&mut self, effect: EffectBundle) {
        self.bundles.push_front(effect);
    }

    pub fn extend(&mut self, other: PendingEffects) {
        self.bundles.extend(other.bundles);
    }

    pub fn is_empty(&self) -> bool {
        self.bundles.is_empty()
    }

    pub fn target_for_option(&self, db: &Database, option: usize) -> Option<Selected> {
        self.bundles.front().and_then(|first| {
            first.effects[self.resolving]
                .effect
                .as_ref()
                .unwrap()
                .target_for_option(db, first.source, &first.selected, &first.modes, option)
        })
    }

    pub fn priority(&self, db: &Database) -> Owner {
        self.bundles
            .front()
            .map(|first| {
                first.effects[self.resolving]
                    .effect
                    .as_ref()
                    .unwrap()
                    .priority(db, first.source)
            })
            .unwrap_or_else(|| db.turn.priority_player())
    }

    pub fn description(&self, db: &Database) -> String {
        self.bundles
            .front()
            .map(|first| {
                first.effects[self.resolving]
                    .effect
                    .as_ref()
                    .unwrap()
                    .description(db, first.source, &first.selected, &first.modes)
            })
            .unwrap_or_default()
    }

    pub fn resolve(&mut self, db: &mut Database, option: Option<usize>) -> SelectionResult {
        if let Some(first) = self.bundles.front_mut() {
            match first.effects[self.resolving]
                .effect
                .as_mut()
                .unwrap()
                .select(
                    db,
                    first.source,
                    option,
                    &mut first.selected,
                    &mut first.modes,
                ) {
                SelectionResult::Complete => {
                    self.resolving += 1;
                    if self.resolving == first.effects.len() {
                        self.resolving = 0;
                        let mut first = self.bundles.pop_front().unwrap();
                        for mut effect in first.effects {
                            effect.effect.as_mut().unwrap().apply(
                                db,
                                self,
                                first.source,
                                &mut first.selected,
                                &first.modes,
                                false,
                            );
                        }
                    }
                    SelectionResult::TryAgain
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
            .map(|front| {
                let effect = &front.effects[self.resolving];
                effect.effect.as_ref().unwrap().options(
                    db,
                    front.source,
                    &front.selected,
                    &front.modes,
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
            count::Count::XCost(_) => db[source.unwrap()].x_is as i32,
        }
    }
}

fn handle_replacements<T: Into<effect::Effect>>(
    db: &Database,
    pending: &mut PendingEffects,
    source: Option<CardId>,
    replacing: Replacing,
    effect: T,
    passes_restrictions: impl Fn(CardId, &[Restriction]) -> bool,
) {
    let replacements = db.replacement_abilities_watching(replacing);
    let selected = replacements
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

    pending.push_back(EffectBundle {
        selected: SelectedStack::new(selected),
        effects: vec![Effect {
            effect: Some(
                ReorderSelected {
                    associated_effect: protobuf::MessageField::some(Effect {
                        effect: Some(effect.into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                }
                .into(),
            ),
            ..Default::default()
        }],
        source,
        ..Default::default()
    });
}
