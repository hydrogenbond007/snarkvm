use crate::builtins::cmp::columns::*;
use itertools::Itertools;

use crate::stark::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::stark::cross_table_lookup::Column;
use crate::stark::stark::Stark;
use crate::stark::vars::{StarkEvaluationTargets, StarkEvaluationVars};
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use std::marker::PhantomData;

#[derive(Copy, Clone, Default)]
pub struct CmpStark<F, const D: usize> {
    pub _phantom: PhantomData<F>,
}

impl<F: RichField, const D: usize> CmpStark<F, D> {
    const BASE: usize = 1 << 16;
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for CmpStark<F, D> {
    const COLUMNS: usize = COL_NUM_CMP;

    // Since op0 is in [0, U32), op1 is in [0, U32)
    // op0, op1 are all field elements
    // if op0 >= op1 is true
    //    diff = op0 - op1  is in [0, U32)
    // if op0 >= op1 is false
    //    diff = op0 - op1 < 0; as this is in finite field, so diff = P + (op0 -
    // op1) As P =  2^64 - 2^32 + 1; op0 - op1 in (-U32, 0)
    // So P + (op0 - op1) > U32
    // so if we Constraint the diff is U32, RC(diff), we could get the GTE relation
    // between op0, op1 The constraints is should be:
    // 1. addition check
    //       op0 = diff + op1
    // 2. rangecheck for diff
    //      RC(diff)
    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: StarkEvaluationVars<FE, P, { COL_NUM_CMP }>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        let op0 = vars.local_values[OP0];
        let op1 = vars.local_values[OP1];
        let diff = vars.local_values[DIFF];

        // Addition check for op0, op1, diff
        yield_constr.constraint(op0 - (op1 + diff));

        let limb_lo = vars.local_values[DIFF_LIMB_LO];
        let limb_hi = vars.local_values[DIFF_LIMB_HI];

        // Addition check for op0, op1, diff
        let base = P::Scalar::from_canonical_usize(Self::BASE);
        let sum = limb_lo + limb_hi * base;

        yield_constr.constraint(diff - sum);

        /*eval_lookups(
            vars,
            yield_constr,
            DIFF_LIMB_LO_PERMUTED,
            FIX_RANGE_CHECK_U16_PERMUTED_LO,
        );
        eval_lookups(
            vars,
            yield_constr,
            DIFF_LIMB_HI_PERMUTED,
            FIX_RANGE_CHECK_U16_PERMUTED_HI,
        );*/
    }

    fn eval_ext_circuit(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, { COL_NUM_CMP }>,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        let op0 = vars.local_values[OP0];
        let op1 = vars.local_values[OP1];
        let diff = vars.local_values[DIFF];

        let op1_diff_sum = builder.add_extension(op1, diff);
        let op0_op1_diff = builder.sub_extension(op0, op1_diff_sum);
        yield_constr.constraint(builder, op0_op1_diff);

        let limb_lo = vars.local_values[DIFF_LIMB_LO];
        let limb_hi = vars.local_values[DIFF_LIMB_HI];

        // Addition check for op0, op1, diff
        let base = builder.constant_extension(F::Extension::from_canonical_usize(Self::BASE));
        let sum = builder.mul_add_extension(limb_hi, base, limb_lo);
        let sum_diff = builder.sub_extension(diff, sum);
        yield_constr.constraint(builder, sum_diff);
    }

    fn constraint_degree(&self) -> usize {
        3
    }
}

// Get the column info for Cross_Lookup<Cpu_table, Bitwise_table>
pub fn ctl_data_with_rangecheck<F: Field>() -> Vec<Column<F>> {
    Column::singles([DIFF]).collect_vec()
}

pub fn ctl_filter_with_rangecheck<F: Field>() -> Column<F> {
    Column::single(FILTER)
}

// Get the column info for Cross_Lookup<Cpu_table, Bitwise_table>
pub fn ctl_data_with_cpu<F: Field>() -> Vec<Column<F>> {
    Column::singles([OP0, OP1]).collect_vec()
}

pub fn ctl_filter_with_cpu<F: Field>() -> Column<F> {
    Column::single(FILTER)
}
