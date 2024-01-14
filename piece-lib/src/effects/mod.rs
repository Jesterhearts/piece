pub(crate) mod apply_then_if_was;
pub(crate) mod battle_cry;
pub(crate) mod battlefield_modifier;
pub(crate) mod cant_attack_this_turn;
pub(crate) mod cascade;
pub(crate) mod controller_discards;
pub(crate) mod controller_draws_cards;
pub(crate) mod controller_loses_life;
pub(crate) mod copy_of_any_creature_non_targeting;
pub(crate) mod copy_spell_or_ability;
pub(crate) mod counter_spell;
pub(crate) mod counter_spell_unless_pay;
pub(crate) mod create_token;
pub(crate) mod create_token_copy;
pub(crate) mod cycling;
pub(crate) mod deal_damage;
pub(crate) mod destroy_each;
pub(crate) mod destroy_target;
pub(crate) mod discover;
pub(crate) mod equip;
pub(crate) mod examine_top_cards;
pub(crate) mod exile_target;
pub(crate) mod exile_target_creature_manifest_top_of_library;
pub(crate) mod exile_target_graveyard;
pub(crate) mod for_each_player_choose_then;
pub(crate) mod foreach_mana_of_source;
pub(crate) mod gain_life;
pub(crate) mod if_then_else;
pub(crate) mod mill;
pub(crate) mod modal;
pub(crate) mod modify_target;
pub(crate) mod multiply_tokens;
pub(crate) mod pay_cost_then;
pub(crate) mod return_from_graveyard_to_battlefield;
pub(crate) mod return_from_graveyard_to_hand;
pub(crate) mod return_from_graveyard_to_library;
pub(crate) mod return_self_to_hand;
pub(crate) mod return_target_to_hand;
pub(crate) mod return_transformed;
pub(crate) mod reveal_each_top_of_library;
pub(crate) mod scry;
pub(crate) mod self_explores;
pub(crate) mod tap_target;
pub(crate) mod tap_this;
pub(crate) mod target_controller_gains_tokens;
pub(crate) mod target_copies_permanent;
pub(crate) mod target_creature_explores;
pub(crate) mod target_gains_counters;
pub(crate) mod target_to_top_of_library;
pub(crate) mod transform;
pub(crate) mod tutor_library;
pub(crate) mod untap_target;
pub(crate) mod untap_this;

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
        ApplyThenIfWas(ApplyThenIfWas),
        BattleCry(BattleCry),
        BattlefieldModifier(BattlefieldModifier),
        CantAttackThisTurn(CantAttackThisTurn),
        Cascade(Cascade),
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
        GainLife(GainLife),
        IfThenElse(IfThenElse),
        Mill(Mill),
        Modal(Modal),
        ModifyTarget(ModifyTarget),
        MultiplyTokens(MultiplyTokens),
        PayCostThen(PayCostThen),
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
        apply_to_self: bool,
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
