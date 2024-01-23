//! The lifetime of an effect:
//! - First the effect is moved into a set of [PendingResults] by calling either
//!   [EffectBehaviors::push_pending_behavior] or [EffectBehaviors::push_behavior_with_targets].
//!     - [EffectBehaviors::push_pending_behavior] is intended to be used when the list of targets
//!       for the effect is unknown. This delegates to the specific effect implementation to decide
//!       how it wants the [PendingResults] to present options to the end user, or if it wants to
//!       choose targets at all. This is typically the entrypoint for most effects.
//!     - [EffectBehaviors::push_behavior_with_targets] is intended to be called when the list of
//!       targets for the effect is known, and the effect should tell [PendingResults] how to handle
//!       resolving the effect.
//! - Then [PendingResults] handles optional target selection for the effect. If targets are
//!   selected, [PendingResults] calls back into [EffectBehaviors::push_behavior_with_targets] to
//!   get the final set of behaviors from the effect.
//! - Then [crate::action_result::ActionResult::apply_action_results] is called on the aggregated
//!   list of actions to take for the effect. These are deferred so that [PendingResults] can
//!   cancel taking action and so that the results of effects don't interfere with the application
//!   of followon effects in the  same batch. This batching can be circumvented with the
//!   [PendingResults::apply_in_stages] flag.

mod apply_then;
mod apply_then_if_was;
mod battle_cry;
mod battlefield_modifier;
mod cant_attack_this_turn;
mod cascade;
mod choose_cast;
mod controller_discards;
mod controller_draws_cards;
mod controller_loses_life;
mod copy_of_any_creature_non_targeting;
mod copy_spell_or_ability;
mod counter_spell;
mod counter_spell_unless_pay;
mod create_token;
mod create_token_copy;
mod cycling;
mod deal_damage;
mod destroy_each;
mod destroy_target;
mod discover;
mod equip;
mod examine_top_cards;
mod exile_target;
mod exile_target_creature_manifest_top_of_library;
mod exile_target_graveyard;
mod for_each_player_choose_then;
mod foreach_mana_of_source;
mod gain_counters;
mod gain_life;
mod if_then_else;
mod mill;
mod modal;
mod modify_target;
mod multiply_tokens;
mod pay_cost_then;
mod rebound;
mod return_from_graveyard_to_battlefield;
mod return_from_graveyard_to_hand;
mod return_from_graveyard_to_library;
mod return_self_to_hand;
mod return_target_to_hand;
mod return_transformed;
mod reveal_each_top_of_library;
mod scry;
mod self_explores;
mod tap_target;
mod tap_this;
mod target_controller_gains_tokens;
mod target_copies_permanent;
mod target_creature_explores;
mod target_gains_counters;
mod target_to_top_of_library;
mod transform;
mod tutor_library;
mod untap_target;
mod untap_this;

use std::{collections::HashSet, vec::IntoIter};

use itertools::Itertools;

use crate::{
    in_play::{CardId, Database},
    log::LogId,
    pending_results::PendingResults,
    player::{Controller, Owner},
    protogen::effects::{Mode, ModifyBattlefield, ReplacementEffect},
    stack::ActiveTarget,
};

#[enum_delegate::implement_for(crate::protogen::effects::effect::Effect,
    enum Effect {
        ApplyThen(ApplyThen),
        ApplyThenIfWas(ApplyThenIfWas),
        BattleCry(BattleCry),
        BattlefieldModifier(BattlefieldModifier),
        CantAttackThisTurn(CantAttackThisTurn),
        Cascade(Cascade),
        ChooseCast(ChooseCast),
        ControllerDiscards(ControllerDiscards),
        ControllerDrawsCards(ControllerDrawsCards),
        ControllerLosesLife(ControllerLosesLife),
        CopyOfAnyCreatureNonTargeting(CopyOfAnyCreatureNonTargeting),
        CopySpellOrAbility(CopySpellOrAbility),
        CounterSpellOrAbility(CounterSpellOrAbility),
        CounterSpellUnlessPay(CounterSpellUnlessPay),
        CreateToken(CreateToken),
        CreateTokenCopy(CreateTokenCopy),
        Cycling(Cycling),
        DealDamage(DealDamage),
        DestroyEach(DestroyEach),
        DestroyTarget(DestroyTarget),
        Discover(Discover),
        Equip(Equip),
        ExamineTopCards(ExamineTopCards),
        ExileTarget(ExileTarget),
        ExileTargetCreatureManifestTopOfLibrary(ExileTargetCreatureManifestTopOfLibrary),
        ExileTargetGraveyard(ExileTargetGraveyard),
        ForEachPlayerChooseThen(ForEachPlayerChooseThen),
        ForEachManaOfSource(ForEachManaOfSource),
        GainCounters(GainCounters),
        GainLife(GainLife),
        IfThenElse(IfThenElse),
        Mill(Mill),
        Modal(Modal),
        ModifyTarget(ModifyTarget),
        MultiplyTokens(MultiplyTokens),
        PayCostThen(PayCostThen),
        Rebound(Rebound),
        ReturnFromGraveyardToBattlefield(ReturnFromGraveyardToBattlefield),
        ReturnFromGraveyardToHand(ReturnFromGraveyardToHand),
        ReturnFromGraveyardToLibrary(ReturnFromGraveyardToLibrary),
        ReturnSelfToHand(ReturnSelfToHand),
        ReturnTargetToHand(ReturnTargetToHand),
        ReturnTransformed(ReturnTransformed),
        RevealEachTopOfLibrary(RevealEachTopOfLibrary),
        Scry(Scry),
        SelfExplores(SelfExplores),
        TapTarget(TapTarget),
        TapThis(TapThis),
        TargetControllerGainsTokens(TargetControllerGainsTokens),
        TargetCopiesPermanent(TargetCopiesPermanent),
        TargetCreatureExplores(TargetCreatureExplores),
        TargetGainsCounters(TargetGainsCounters),
        TargetToTopOfLibrary(TargetToTopOfLibrary),
        Transform(Transform),
        TutorLibrary(TutorLibrary),
        UntapTarget(UntapTarget),
        UntapThis(UntapThis),
    }
)]
pub(crate) trait EffectBehaviors {
    fn choices(&self, db: &Database, targets: &[ActiveTarget]) -> Vec<String> {
        targets
            .iter()
            .map(|target| target.display(db))
            .collect_vec()
    }

    fn modes(&self) -> Vec<Mode> {
        vec![]
    }

    fn is_sorcery_speed(&self) -> bool {
        false
    }

    fn is_equip(&self) -> bool {
        false
    }

    fn cycling(&self) -> bool {
        false
    }

    fn needs_targets(&self, db: &Database, source: CardId) -> usize;

    fn wants_targets(&self, db: &Database, source: CardId) -> usize;

    fn valid_targets(
        &self,
        db: &Database,
        source: CardId,
        log_session: LogId,
        controller: Controller,
        already_chosen: &HashSet<ActiveTarget>,
    ) -> Vec<ActiveTarget> {
        let _ = db;
        let _ = source;
        let _ = log_session;
        let _ = controller;
        let _ = already_chosen;
        vec![]
    }

    fn push_pending_behavior(
        &self,
        db: &mut Database,
        source: CardId,
        controller: Controller,
        results: &mut PendingResults,
    );

    fn push_behavior_from_top_of_library(
        &self,
        db: &Database,
        source: CardId,
        target_card: CardId,
        results: &mut PendingResults,
    ) {
        let _ = db;
        let _ = source;
        let _ = target_card;
        let _ = results;
        unreachable!()
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut Database,
        targets: Vec<ActiveTarget>,
        source: CardId,
        controller: Controller,
        results: &mut PendingResults,
    );

    fn replace_draw(
        &self,
        db: &mut Database,
        player: Owner,
        replacements: &mut IntoIter<(CardId, ReplacementEffect)>,
        controller: Controller,
        count: usize,
        results: &mut PendingResults,
    ) {
        let _ = db;
        let _ = player;
        let _ = replacements;
        let _ = controller;
        let _ = count;
        let _ = results;
        unreachable!()
    }

    fn replace_token_creation(
        &self,
        db: &mut Database,
        source: CardId,
        replacements: &mut IntoIter<(CardId, ReplacementEffect)>,
        token: CardId,
        modifiers: &[ModifyBattlefield],
        results: &mut PendingResults,
    ) {
        let _ = db;
        let _ = source;
        let _ = replacements;
        let _ = token;
        let _ = modifiers;
        let _ = results;
        unreachable!()
    }
}
