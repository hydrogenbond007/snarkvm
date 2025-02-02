use crate::builtins::rangecheck::columns::*;
use itertools::Itertools;

use crate::stark::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::stark::cross_table_lookup::Column;
use crate::stark::lookup::*;
use crate::stark::permutation::*;
use crate::stark::stark::Stark;
use crate::stark::vars::{StarkEvaluationTargets, StarkEvaluationVars};
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use std::marker::PhantomData;

#[derive(Copy, Clone, Default)]
pub struct RangeCheckStark<F, const D: usize> {
    pub _phantom: PhantomData<F>,
}

impl<F: RichField, const D: usize> RangeCheckStark<F, D> {
    const BASE: usize = 1 << 16;
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for RangeCheckStark<F, D> {
    const COLUMNS: usize = COL_NUM_RC;

    // Split U32 into 2 16bit limbs
    // Sumcheck between Val and limbs
    // RC for limbs
    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: StarkEvaluationVars<FE, P, { COL_NUM_RC }>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        let val = vars.local_values[VAL];
        let limb_lo = vars.local_values[LIMB_LO];
        let limb_hi = vars.local_values[LIMB_HI];

        // Addition check for op0, op1, diff
        let base = P::Scalar::from_canonical_usize(Self::BASE);
        let sum = limb_lo + limb_hi * base;

        yield_constr.constraint(val - sum);

        eval_lookups(
            vars,
            yield_constr,
            LIMB_LO_PERMUTED,
            FIX_RANGE_CHECK_U16_PERMUTED_LO,
        );
        eval_lookups(
            vars,
            yield_constr,
            LIMB_HI_PERMUTED,
            FIX_RANGE_CHECK_U16_PERMUTED_HI,
        );
    }

    fn eval_ext_circuit(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, { COL_NUM_RC }>,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        let val = vars.local_values[VAL];
        let limb_lo = vars.local_values[LIMB_LO];
        let limb_hi = vars.local_values[LIMB_HI];

        // Addition check for op0, op1, diff
        let base = builder.constant_extension(F::Extension::from_canonical_usize(Self::BASE));
        let sum = builder.mul_add_extension(limb_hi, base, limb_lo);
        let val_sum_diff = builder.sub_extension(val, sum);
        yield_constr.constraint(builder, val_sum_diff);

        eval_lookups_circuit(
            builder,
            vars,
            yield_constr,
            LIMB_LO_PERMUTED,
            FIX_RANGE_CHECK_U16_PERMUTED_LO,
        );
        eval_lookups_circuit(
            builder,
            vars,
            yield_constr,
            LIMB_HI_PERMUTED,
            FIX_RANGE_CHECK_U16_PERMUTED_HI,
        );
    }

    fn constraint_degree(&self) -> usize {
        3
    }

    fn permutation_pairs(&self) -> Vec<PermutationPair> {
        vec![
            PermutationPair::singletons(LIMB_LO, LIMB_LO_PERMUTED),
            PermutationPair::singletons(LIMB_HI, LIMB_HI_PERMUTED),
            PermutationPair::singletons(FIX_RANGE_CHECK_U16, FIX_RANGE_CHECK_U16_PERMUTED_LO),
            PermutationPair::singletons(FIX_RANGE_CHECK_U16, FIX_RANGE_CHECK_U16_PERMUTED_HI),
        ]
    }
}

pub fn ctl_data_memory<F: Field>() -> Vec<Column<F>> {
    Column::singles([VAL]).collect_vec()
}

pub fn ctl_filter_memory<F: Field>() -> Column<F> {
    Column::single(MEMORY_FILTER)
}

// Get the column info for Cross_Lookup<Cpu_table, Bitwise_table>
pub fn ctl_data_with_cmp<F: Field>() -> Vec<Column<F>> {
    Column::singles([VAL]).collect_vec()
}

pub fn ctl_filter_with_cmp<F: Field>() -> Column<F> {
    Column::single(CMP_FILTER)
}

// Get the column info for Cross_Lookup<Cpu_table, Bitwise_table>
pub fn ctl_data_with_cpu<F: Field>() -> Vec<Column<F>> {
    Column::singles([VAL]).collect_vec()
}

pub fn ctl_filter_with_cpu<F: Field>() -> Column<F> {
    Column::single(CPU_FILTER)
}

// Get the column info for Cross_Lookup<Cpu_table, Bitwise_table>
/*pub fn ctl_data_with_rangecheck_fixed<F: Field>() -> Vec<Column<F>> {
    let mut res = Column::singles([LIMB_LO, LIMB_HI]).collect_vec();
    res
}
pub fn ctl_filter_with_rangecheck_fixed<F: Field>() -> Column<F> {
    Column::one()
}*/
