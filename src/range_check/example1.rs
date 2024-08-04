/// This helper checks that the value witnessed in a given cell is within a given range
///
///  value | q_range_check
///     v  |        1

use std::marker::PhantomData;

use halo2_proofs::{
    arithmetic::FieldExt, circuit::*, plonk::*, poly::Rotation
};

#[derive(Clone, Debug)]
struct RnageCheckConfig<F: FieldExt, const RANGE: usize> {
    value: Column<Advice>,
    q_range_check: Selector,
    _marker: PhantomData<F>,
}

impl <F: FieldExt, const RANGE: usize> RnageCheckConfig<F, RANGE> {        
    fn configure(
        meta: &mut ConstraintSystem<F>,
        value: Column<Advice>
    ) -> Self {
        // Toggle the range check constaint
        let q_range_check = meta.selector();

        const RANGE: usize = 100;

        // Range check gate
        // For a value v and a range R, check that v < R
        // v * (1-v) * (2-v) * ....... * (R - 1 - v)
        meta.create_gate("range_check", |meta| {
            let q = meta.query_selector(q_range_check);
            let value = meta.query_advice(value, Rotation::cur());

            let range_check = |range: usize, value: Expression<F>| {
                (0..range).fold(value.clone(), |expr, i|{
                    expr * (Expression::Constant(F::from(i as u64)) - value.clone())
                })
            };

            Constraints::with_selector(q, [("range check", range_check(RANGE, value))])
        });

        Self {
            value,
            q_range_check,
            _marker: PhantomData,
        }
      
    }

    fn assign(
        &self,
        mut layouter: impl Layouter<F>,
        value: Value<F>,
    ) -> Result<(), Error> {
        layouter.assign_region(|| "Assign Value", |mut region| {
             // enable q_range_check
        let _ = self.q_range_check.enable(&mut region, 0);

        // Assign given value
        let _ = region.assign_advice(|| "value", self.value, 0, || value);

        Ok(())
        })
    }
    
}

#[cfg(test)]
mod tests {
    use halo2_proofs::{
        arithmetic::FieldExt, circuit::{self, *}, dev::{FailureLocation, MockProver, VerifyFailure}, pasta::Fp, plonk::*, poly::Rotation
    };
use super::*;
#[derive(Default)]
struct MyCircuit<F: FieldExt, const RANGE: usize> {
    value: Value<F>,
}

impl<F: FieldExt, const RANGE: usize> Circuit<F> for MyCircuit<F, RANGE> {
    type Config = RnageCheckConfig<F, RANGE>;
    type FloorPlanner = SimpleFloorPlanner;
    
    fn without_witnesses(&self) -> Self {
        Self::default()
    }
    
    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let value = meta.advice_column();
        RnageCheckConfig::configure(meta, value)
    }
    
    fn synthesize(&self, 
        config: Self::Config, 
        mut layouter: impl Layouter<F>
     ) -> Result<(), Error> {
        config.assign(layouter.namespace(|| "Assign Value"), self.value)?;

        Ok(())
    }
}
#[test]
fn test_range_check() {
    let k = 4;
    const RANGE: usize = 8; // 3-bit value

    // Successful cases
    for i in 0..RANGE{
        let circuit = MyCircuit::<Fp, RANGE> {
            value: Value::known(Fp::from(i as u64).into()),
        };

    let prover = MockProver::run(k, &circuit, vec![]).unwrap();
    prover.assert_satisfied();
    }
    // Out-of-range `value = 8`
    {
        let circuit = MyCircuit::<Fp, RANGE> {
            value: Value::known(Fp::from(RANGE as u64).into()),
        };
        let prover = MockProver::run(k, &circuit, vec![]).unwrap();
        assert_eq!(
            prover.verify(),
            Err(vec![VerifyFailure::ConstraintNotSatisfied {
                constraint: ((0, "range check").into(), 0, "range check").into(),
                location: FailureLocation::InRegion {
                    region: (0, "Assign value").into(),
                    offset: 0
                },
                cell_values: vec![(((Any::Advice, 0).into(), 0).into(), "0x8".to_string())]
            }])
        );
    }
}
}
    