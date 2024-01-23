pub(crate) mod add_ability_to_stack;
pub(crate) mod add_counters;
pub(crate) mod add_to_battlefield;
pub(crate) mod add_to_battlefield_from_library;
pub(crate) mod add_to_battlefield_skip_replacement_effects;
pub(crate) mod add_to_battlefield_skip_replacement_effects_from_exile;
pub(crate) mod add_to_battlefield_skip_replacement_effects_from_library;
pub(crate) mod apply_aura_to_target;
pub(crate) mod apply_to_battlefield;
pub(crate) mod ban_attacking;
pub(crate) mod cascade;
pub(crate) mod cascade_exile_to_bottom_of_library;
pub(crate) mod cast_card;
pub(crate) mod clone_card;
pub(crate) mod copy_ability;
pub(crate) mod copy_card_in_stack;
pub(crate) mod create_token;
pub(crate) mod create_token_copy_of;
pub(crate) mod damage_target;
pub(crate) mod declare_attackers;
pub(crate) mod destroy_each;
pub(crate) mod destroy_target;
pub(crate) mod discard;
pub(crate) mod discard_cards;
pub(crate) mod discover;
pub(crate) mod draw_cards;
pub(crate) mod examine_top_cards;
pub(crate) mod exile_graveyard;
pub(crate) mod exile_target;
pub(crate) mod explore;
pub(crate) mod for_each_mana_of_source;
pub(crate) mod gain_life;
pub(crate) mod gain_mana;
pub(crate) mod hand_from_battlefield;
pub(crate) mod if_was_then;
pub(crate) mod lose_life;
pub(crate) mod manifest_top_of_library;
pub(crate) mod mill;
pub(crate) mod modify_creatures;
pub(crate) mod move_from_library_to_bottom_of_library;
pub(crate) mod move_from_library_to_graveyard;
pub(crate) mod move_from_library_to_top_of_library;
pub(crate) mod move_to_hand_from_library;
pub(crate) mod permanent_to_graveyard;
pub(crate) mod player_loses;
pub(crate) mod remove_counters;
pub(crate) mod return_from_battlefield_to_library;
pub(crate) mod return_from_graveyard_to_battlefield;
pub(crate) mod return_from_graveyard_to_hand;
pub(crate) mod return_from_graveyard_to_library;
pub(crate) mod return_transformed;
pub(crate) mod reveal_card;
pub(crate) mod reveal_each_top_of_library;
pub(crate) mod scry;
pub(crate) mod shuffle;
pub(crate) mod spell_countered;
pub(crate) mod spend_mana;
pub(crate) mod stack_to_graveyard;
pub(crate) mod tap_permanent;
pub(crate) mod then_apply;
pub(crate) mod transform;
pub(crate) mod untap;
pub(crate) mod update_stack_entries;

use std::vec::IntoIter;

use itertools::Itertools;
use tracing::Level;

use crate::{
    action_result::{
        add_ability_to_stack::AddAbilityToStack, add_counters::AddCounters,
        add_to_battlefield::AddToBattlefield,
        add_to_battlefield_from_library::AddToBattlefieldFromLibrary,
        add_to_battlefield_skip_replacement_effects::AddToBattlefieldSkipReplacementEffects,
        add_to_battlefield_skip_replacement_effects_from_exile::AddToBattlefieldSkipReplacementEffectsFromExile,
        add_to_battlefield_skip_replacement_effects_from_library::AddToBattlefieldSkipReplacementEffectsFromLibrary,
        apply_aura_to_target::ApplyAuraToTarget, apply_to_battlefield::ApplyToBattlefield,
        ban_attacking::BanAttacking, cascade::Cascade,
        cascade_exile_to_bottom_of_library::CascadeExileToBottomOfLibrary, cast_card::CastCard,
        clone_card::CloneCard, copy_ability::CopyAbility, copy_card_in_stack::CopyCardInStack,
        create_token::CreateToken, create_token_copy_of::CreateTokenCopyOf,
        damage_target::DamageTarget, declare_attackers::DeclareAttackers,
        destroy_each::DestroyEach, destroy_target::DestroyTarget, discard::Discard,
        discard_cards::DiscardCards, discover::Discover, draw_cards::DrawCards,
        examine_top_cards::ExamineTopCards, exile_graveyard::ExileGraveyard,
        exile_target::ExileTarget, explore::Explore, for_each_mana_of_source::ForEachManaOfSource,
        gain_life::GainLife, gain_mana::GainMana, hand_from_battlefield::HandFromBattlefield,
        if_was_then::IfWasThen, lose_life::LoseLife, manifest_top_of_library::ManifestTopOfLibrary,
        mill::Mill, modify_creatures::ModifyCreatures,
        move_from_library_to_bottom_of_library::MoveFromLibraryToBottomOfLibrary,
        move_from_library_to_graveyard::MoveFromLibraryToGraveyard,
        move_from_library_to_top_of_library::MoveFromLibraryToTopOfLibrary,
        move_to_hand_from_library::MoveToHandFromLibrary,
        permanent_to_graveyard::PermanentToGraveyard, player_loses::PlayerLoses,
        remove_counters::RemoveCounters,
        return_from_battlefield_to_library::ReturnFromBattlefieldToLibrary,
        return_from_graveyard_to_battlefield::ReturnFromGraveyardToBattlefield,
        return_from_graveyard_to_hand::ReturnFromGraveyardToHand,
        return_from_graveyard_to_library::ReturnFromGraveyardToLibrary,
        return_transformed::ReturnTransformed, reveal_card::RevealCard,
        reveal_each_top_of_library::RevealEachTopOfLibrary, scry::Scry, shuffle::Shuffle,
        spell_countered::SpellCountered, spend_mana::SpendMana,
        stack_to_graveyard::StackToGraveyard, tap_permanent::TapPermanent, then_apply::ThenApply,
        transform::Transform, untap::Untap, update_stack_entries::UpdateStackEntries,
    },
    battlefield::Battlefields,
    effects::EffectBehaviors,
    in_play::{CardId, Database, ModifierId},
    log::{Log, LogEntry, LogId},
    pending_results::PendingResults,
    protogen::{
        effects::{BattlefieldModifier, Duration, ModifyBattlefield, ReplacementEffect},
        triggers::TriggerSource,
    },
    stack::Stack,
};

#[enum_delegate::register]
pub(crate) trait Action {
    fn apply(&self, db: &mut Database) -> PendingResults;
}

#[derive(Debug, Clone)]
#[enum_delegate::implement(Action)]
pub(crate) enum ActionResult {
    AddAbilityToStack(AddAbilityToStack),
    AddCounters(AddCounters),
    AddToBattlefield(AddToBattlefield),
    AddToBattlefieldFromLibrary(AddToBattlefieldFromLibrary),
    AddToBattlefieldSkipReplacementEffects(AddToBattlefieldSkipReplacementEffects),
    AddToBattlefieldSkipReplacementEffectsFromExile(
        AddToBattlefieldSkipReplacementEffectsFromExile,
    ),
    AddToBattlefieldSkipReplacementEffectsFromLibrary(
        AddToBattlefieldSkipReplacementEffectsFromLibrary,
    ),
    ApplyAuraToTarget(ApplyAuraToTarget),
    ApplyToBattlefield(ApplyToBattlefield),
    BanAttacking(BanAttacking),
    Cascade(Cascade),
    CascadeExileToBottomOfLibrary(CascadeExileToBottomOfLibrary),
    CastCard(CastCard),
    CloneCard(CloneCard),
    CopyAbility(CopyAbility),
    CopyCardInStack(CopyCardInStack),
    CreateToken(CreateToken),
    CreateTokenCopyOf(CreateTokenCopyOf),
    DamageTarget(DamageTarget),
    DeclareAttackers(DeclareAttackers),
    DestroyEach(DestroyEach),
    DestroyTarget(DestroyTarget),
    Discard(Discard),
    DiscardCards(DiscardCards),
    Discover(Discover),
    DrawCards(DrawCards),
    ExamineTopCards(ExamineTopCards),
    ExileGraveyard(ExileGraveyard),
    ExileTarget(ExileTarget),
    Explore(Explore),
    ForEachManaOfSource(ForEachManaOfSource),
    GainLife(GainLife),
    GainMana(GainMana),
    HandFromBattlefield(HandFromBattlefield),
    IfWasThen(IfWasThen),
    LoseLife(LoseLife),
    ManifestTopOfLibrary(ManifestTopOfLibrary),
    Mill(Mill),
    ModifyCreatures(ModifyCreatures),
    MoveFromLibraryToBottomOfLibrary(MoveFromLibraryToBottomOfLibrary),
    MoveFromLibraryToGraveyard(MoveFromLibraryToGraveyard),
    MoveFromLibraryToTopOfLibrary(MoveFromLibraryToTopOfLibrary),
    MoveToHandFromLibrary(MoveToHandFromLibrary),
    PlayerLoses(PlayerLoses),
    PermanentToGraveyard(PermanentToGraveyard),
    RemoveCounters(RemoveCounters),
    ReturnFromBattlefieldToLibrary(ReturnFromBattlefieldToLibrary),
    ReturnFromGraveyardToBattlefield(ReturnFromGraveyardToBattlefield),
    ReturnFromGraveyardToHand(ReturnFromGraveyardToHand),
    ReturnFromGraveyardToLibrary(ReturnFromGraveyardToLibrary),
    ReturnTransformed(ReturnTransformed),
    RevealCard(RevealCard),
    RevealEachTopOfLibrary(RevealEachTopOfLibrary),
    Scry(Scry),
    Shuffle(Shuffle),
    SpellCountered(SpellCountered),
    SpendMana(SpendMana),
    StackToGraveyard(StackToGraveyard),
    TapPermanent(TapPermanent),
    ThenApply(ThenApply),
    Transform(Transform),
    Untap(Untap),
    UpdateStackEntries(UpdateStackEntries),
}

impl ActionResult {
    #[instrument(skip(db), level = Level::DEBUG)]
    pub(crate) fn apply_action_results(
        db: &mut Database,
        results: &[ActionResult],
    ) -> PendingResults {
        let mut pending = PendingResults::default();

        for result in results.iter() {
            pending.extend(result.apply(db));
        }

        let entries = Log::current_session(db).to_vec();
        for (listener, trigger) in db.active_triggers_of_source(TriggerSource::ONE_OR_MORE_TAPPED) {
            if entries.iter().any(|entry| {
                let (_, LogEntry::Tapped { card }) = entry else {
                    return false;
                };

                card.passes_restrictions(
                    db,
                    LogId::current(db),
                    listener,
                    &trigger.trigger.restrictions,
                )
            }) {
                pending.extend(Stack::move_trigger_to_stack(db, listener, trigger));
            }
        }

        for card in db.cards.keys().copied().collect_vec() {
            card.apply_modifiers_layered(db);
        }

        pending
    }
}

#[instrument(skip(db, modifiers, results))]
pub(crate) fn create_token_copy_with_replacements(
    db: &mut Database,
    source: CardId,
    copying: CardId,
    modifiers: &[ModifyBattlefield],
    replacements: &mut IntoIter<(CardId, ReplacementEffect)>,
    results: &mut PendingResults,
) {
    let mut replaced = false;
    if replacements.len() > 0 {
        while let Some((source, replacement)) = replacements.next() {
            if !source.passes_restrictions(
                db,
                LogId::current(db),
                source,
                &source.faceup_face(db).restrictions,
            ) || !copying.passes_restrictions(
                db,
                LogId::current(db),
                source,
                &replacement.restrictions,
            ) {
                continue;
            }

            debug!("Replacing token creation");

            replaced = true;
            for effect in replacement.effects.iter() {
                effect.effect.as_ref().unwrap().replace_token_creation(
                    db,
                    source,
                    replacements,
                    copying,
                    modifiers,
                    results,
                );
            }
            break;
        }
    }

    if !replaced {
        debug!("Creating token");
        let token = copying.token_copy_of(db, db[source].controller);
        for modifier in modifiers.iter() {
            let modifier = ModifierId::upload_temporary_modifier(
                db,
                token,
                BattlefieldModifier {
                    modifier: protobuf::MessageField::some(modifier.clone()),
                    duration: Duration::UNTIL_SOURCE_LEAVES_BATTLEFIELD.into(),
                    ..Default::default()
                },
            );
            modifier.activate(&mut db.modifiers);

            token.apply_modifier(db, modifier);
        }

        token.apply_modifiers_layered(db);
        results.extend(Battlefields::add_from_stack_or_hand(db, token, None));
    }
}
