//! Operator-confirmation types for the hire gate's role-contract axis
//! (DEC-025 #1 + #4).
//!
//! Extracted from `hire_gate.rs` to keep that file under the per-file LOC
//! budget. Owns [`ContractConfirmation`] (the operator's confirmation state
//! for a teammate's role contract) and [`ContractGateInput`] (the contract +
//! its confirmation, the 4th input to [`crate::GateVerdict::compose`]).

use crate::pipeline::GenesisPlan;
use crate::role_contract::RoleContract;

/// Whether the operator has confirmed the teammate's role contract (DEC-025
/// #1 + #4). Mirrors [`crate::SafetySignal`]'s honest tri-state so the
/// composite verdict can tell "the operator confirmed THIS contract" apart
/// from "no confirmation yet" (hold for review) and "a confirmation was
/// offered but it did not match the teammate's truthful contract" (a hard
/// block — a forged or stale confirmation must NEVER activate a teammate).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractConfirmation {
    /// The operator echoed back the digest of the teammate's truthful,
    /// freshly-derived role contract. Activation may proceed.
    Confirmed,
    /// No confirmation digest was supplied. Human-review-before-activation is
    /// ON by default (DEC-025 #1): hold in `needs_review` until the operator
    /// confirms the surfaced contract. NOT a pass, NOT a hard block.
    Unconfirmed,
    /// A confirmation digest was supplied but it did NOT match the digest of
    /// the contract re-derived from the teammate's bound capabilities. The
    /// confirmation is stale (capabilities changed) or forged — a hard fail,
    /// because activating here would bind a confirmation to a teammate the
    /// operator never actually saw.
    Mismatch {
        /// The digest the operator supplied.
        supplied: String,
        /// The digest of the teammate's actual, truthful contract.
        expected: String,
    },
}

/// The role-contract input to [`crate::GateVerdict::compose`]: the teammate's
/// truthful contract (derived from its bound capabilities) plus the
/// operator's confirmation state for it.
#[derive(Debug, Clone)]
pub struct ContractGateInput {
    /// The teammate's truthful role contract, or `None` when no manifest was
    /// available to derive one (the gate then cannot pass).
    pub role_contract: Option<RoleContract>,
    /// The operator's confirmation state for the contract.
    pub confirmation: ContractConfirmation,
}

impl ContractGateInput {
    /// Build the gate input by deriving the truthful contract from `plan`'s
    /// manifest and resolving the operator's `supplied_digest` (if any)
    /// against it.
    ///
    /// Resolution (fail-closed):
    /// - no `supplied_digest` → [`ContractConfirmation::Unconfirmed`] (hold);
    /// - `supplied_digest` equals the truthful contract's digest →
    ///   [`ContractConfirmation::Confirmed`];
    /// - `supplied_digest` differs → [`ContractConfirmation::Mismatch`]
    ///   (hard block — the operator confirmed something other than what the
    ///   teammate actually is).
    ///
    /// # Examples
    ///
    /// ```
    /// # fn demo() -> Result<(), argo_genesis::GenesisError> {
    /// use argo_genesis::{ContractConfirmation, ContractGateInput, RoleContract};
    /// let plan = argo_genesis::plan("be my marketing intern, run weekly campaigns", "maya")?;
    /// let truthful = RoleContract::derive_from_manifest(&plan.manifest).digest();
    /// // No digest → unconfirmed (held for review).
    /// let held = ContractGateInput::from_plan(&plan, None);
    /// assert!(matches!(held.confirmation, ContractConfirmation::Unconfirmed));
    /// // Correct digest → confirmed.
    /// let ok = ContractGateInput::from_plan(&plan, Some(&truthful));
    /// assert!(matches!(ok.confirmation, ContractConfirmation::Confirmed));
    /// # Ok(()) }
    /// ```
    #[must_use]
    pub fn from_plan(plan: &GenesisPlan, supplied_digest: Option<&str>) -> Self {
        let contract = RoleContract::derive_from_manifest(&plan.manifest);
        Self::resolve(Some(contract), supplied_digest)
    }

    /// Resolve a confirmation against an already-derived contract. Shared by
    /// the plan path and the on-disk fence path so both decide confirmation
    /// identically.
    #[must_use]
    pub fn resolve(contract: Option<RoleContract>, supplied_digest: Option<&str>) -> Self {
        let confirmation = match (contract.as_ref(), supplied_digest) {
            (_, None) => ContractConfirmation::Unconfirmed,
            (Some(c), Some(supplied)) => {
                let expected = c.digest();
                if expected == supplied.trim() {
                    ContractConfirmation::Confirmed
                } else {
                    ContractConfirmation::Mismatch {
                        supplied: supplied.trim().to_string(),
                        expected,
                    }
                }
            }
            // A digest supplied with no contract to check it against is a
            // mismatch by construction (cannot confirm what we can't derive).
            (None, Some(supplied)) => ContractConfirmation::Mismatch {
                supplied: supplied.trim().to_string(),
                expected: String::new(),
            },
        };
        Self {
            role_contract: contract,
            confirmation,
        }
    }

    /// The truthful contract's digest, or empty when there is no contract.
    #[must_use]
    pub(crate) fn digest(&self) -> String {
        self.role_contract
            .as_ref()
            .map(RoleContract::digest)
            .unwrap_or_default()
    }
}
