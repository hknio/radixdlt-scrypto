use super::authorization::{
    HardAuthRule,
    MethodAuthorization,
};
use crate::types::*;
use radix_engine_interface::blueprints::resource::*;

fn soft_to_hard_auth_rule(
    auth_rule: &AccessRuleNode,
) -> HardAuthRule {
    match auth_rule {
        AccessRuleNode::ProofRule(proof_rule) => HardAuthRule::ProofRule(proof_rule.clone()),
        AccessRuleNode::AnyOf(rules) => {
            let hard_rules = rules
                .iter()
                .map(|r| soft_to_hard_auth_rule(r))
                .collect();
            HardAuthRule::AnyOf(hard_rules)
        }
        AccessRuleNode::AllOf(rules) => {
            let hard_rules = rules
                .iter()
                .map(|r| soft_to_hard_auth_rule(r))
                .collect();
            HardAuthRule::AllOf(hard_rules)
        }
    }
}

/// Converts an `AccessRule` into a `MethodAuthorization`, with the given context of
/// Scrypto value and schema.
///
/// This method assumes that the value matches with the schema.
pub fn convert(
    method_auth: &AccessRule,
) -> MethodAuthorization {
    match method_auth {
        AccessRule::Protected(auth_rule) => MethodAuthorization::Protected(soft_to_hard_auth_rule(
            auth_rule
        )),
        AccessRule::AllowAll => MethodAuthorization::AllowAll,
        AccessRule::DenyAll => MethodAuthorization::DenyAll,
    }
}
