use std::env;
use group::ff::Field;
use std::marker::PhantomData;
use halo2_proofs::circuit::{AssignedCell, Value};
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::*,
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Fixed, Instance, Selector},
    poly::Rotation,
};

// specify necessary columns in the main table
#[derive(Clone, Debug)]
struct ArithmeticConfig {
    advice: [Column<Advice>; 3],
    instance: Column<Instance>,
    constant: Column<Fixed>,

    // selectors
    s_add: Selector,
    s_mul: Selector,
    s_add_c: Selector,
    s_mul_c: Selector,
}

struct ArithmeticChip<F: FieldExt> {
    config: ArithmeticConfig,
    _marker: PhantomData<F>,
}

impl<F: FieldExt> ArithmeticChip<F> {
    fn construct(config: ArithmeticConfig) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }

    fn configure(
        meta: &mut ConstraintSystem<F>,
        advice: [Column<Advice>; 3],
        instance: Column<Instance>,
        constant: Column<Fixed>,
    ) -> ArithmeticConfig {
        // specify columns used for proving copy constraints
        meta.enable_equality(instance);
        meta.enable_constant(constant);
        for column in &advice {
            meta.enable_equality(*column);
        }

        // extract columns with respect to selectors
        let s_add = meta.selector();
        let s_mul = meta.selector();
        let s_add_c = meta.selector();
        let s_mul_c = meta.selector();

        // Define our multiplication gate!
        meta.create_gate("mul", |meta| {
            let lhs = meta.query_advice(advice[0], Rotation::cur());
            let rhs = meta.query_advice(advice[1], Rotation::cur());
            let out = meta.query_advice(advice[2], Rotation::cur());
            let s_mul = meta.query_selector(s_mul);

            vec![s_mul * (lhs * rhs - out)]
        });

        // Define our addition gate!
        meta.create_gate("add", |meta| {
            let lhs = meta.query_advice(advice[0], Rotation::cur());
            let rhs = meta.query_advice(advice[1], Rotation::cur());
            let out = meta.query_advice(advice[2], Rotation::cur());
            let s_add = meta.query_selector(s_add);

            vec![s_add * (lhs * rhs - out)]
        });
        
        // define addition with constant gate
        meta.create_gate("add with constant", |meta| {
            let s_add_c = meta.query_selector(s_add_c);
            let lhs = meta.query_advice(advice[0], Rotation::cur());
            let fixed = meta.query_fixed(constant, Rotation::cur());
            let out = meta.query_advice(advice[2], Rotation::cur());
            vec![s_add_c * (lhs + fixed - out)]
        });

        // define multiplication with constant gate
        meta.create_gate("mul with constant", |meta| {
            let s_mul_c = meta.query_selector(s_mul_c);
            let lhs = meta.query_advice(advice[0], Rotation::cur());
            let fixed = meta.query_fixed(constant, Rotation::cur());
            let out = meta.query_advice(advice[2], Rotation::cur());
            vec![s_mul_c * (lhs * fixed - out)]
        });

        ArithmeticConfig {
            advice,
            instance,
            constant,
            s_mul,
            s_add,
            s_add_c,
            s_mul_c
        }
    }
}


// ANCHOR: chip-impl
impl<F: FieldExt> Chip<F> for ArithmeticChip<F> {
    type Config = ArithmeticConfig;
    type Loaded = ();

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn loaded(&self) -> &Self::Loaded {
        &()
    }
}
// ANCHOR_END: chip-impl

#[derive(Default)]
struct MyCircuit<F: FieldExt> {
    u: Value<F>,
    v: Value<F>,
}

//#[derive(Clone)]
//struct Number<F: Field>(AssignedCell<F, F>);

impl<F: FieldExt> Circuit<F> for MyCircuit<F> {
//    type Num = Number<F>;

    type Config = ArithmeticConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let advice = [meta.advice_column(), meta.advice_column(), meta.advice_column()];
        let instance = meta.instance_column();
        let constant = meta.fixed_column();
        ArithmeticChip::configure(meta, advice, instance, constant)
    }

    fn synthesize(
        &self, config: Self::Config, mut layouter: impl Layouter<F>
    ) -> Result<(), Error> {
        // handling multiplication region
        let t1 = self.u * self.u;
        let t2 = self.u * self.v;
        let t3 = t2 * Value::known(F::from(3));

        // define multiplication region
        let (
            (x_a1, x_b1, x_c1),
            (x_a2, x_b2, x_c2),
            (x_a3, x_c3)
        ) = layouter.assign_region(
            || "multiplication region",
            |mut region| {
                // first row
                config.s_mul.enable(&mut region, 0)?;
                let x_a1 = region.assign_advice(|| "x_a1",
                    config.advice[0].clone(), 0, || self.u)?;
                let x_b1 = region.assign_advice(|| "x_b1",
                    config.advice[1].clone(), 0, || self.u)?;
                let x_c1 = region.assign_advice(|| "x_c1",
                    config.advice[2].clone(), 0, || t1)?;

                // second row
                config.s_mul.enable(&mut region, 1)?;
                let x_a2 = region.assign_advice(|| "x_a2",
                    config.advice[0].clone(), 1, || self.u)?;
                let x_b2 = region.assign_advice(|| "x_b2",
                    config.advice[1].clone(), 1, || self.v)?;
                let x_c2 = region.assign_advice(|| "x_c2",
                    config.advice[2].clone(), 1, || t2)?;

                // third row
                config.s_mul_c.enable(&mut region, 2)?;
                let x_a3 = region.assign_advice(|| "x_a3",
                    config.advice[0].clone(), 2, || t2)?;
                region.assign_fixed(|| "constant 3",
                    config.constant.clone(), 2, || Value::known(F::from(3)))?;
                let x_c3 = region.assign_advice(|| "x_c3",
                    config.advice[2].clone(), 2, || t3)?;

                Ok((
                    (x_a1.cell(), x_b1.cell(), x_c1.cell()),
                    (x_a2.cell(), x_b2.cell(), x_c2.cell()),
                    (x_a3.cell(), x_c3.cell())
                ))
            }
        )?;

        let t4 = t1 + t3;
        let t5 = t4 + self.v;
        let t6 = t5 + Value::known(F::from(5));

        // define addition region
        let (
            (x_a4, x_b4, x_c4),
            (x_a5, x_b5, x_c5),
            (x_a6, x_c6)
        ) = layouter.assign_region(
            || "addition region",
            |mut region| {
                // first row
                config.s_add.enable(&mut region, 0)?;
                let x_a4 = region.assign_advice(|| "x_a4",
                    config.advice[0].clone(), 0, || t1)?;
                let x_b4 = region.assign_advice(|| "x_b4",
                    config.advice[1].clone(), 0, || t3)?;
                let x_c4 = region.assign_advice(|| "x_c4",
                    config.advice[2].clone(), 0, || t4)?;

                // second row
                config.s_add.enable(&mut region, 1)?;
                let x_a5 = region.assign_advice(|| "x_a5",
                    config.advice[0].clone(), 1, || t4)?;
                let x_b5 = region.assign_advice(|| "x_b5",
                    config.advice[1].clone(), 1, || self.v)?;
                let x_c5 = region.assign_advice(|| "x_c5",
                    config.advice[2].clone(), 1, || t5)?;

                // third row
                config.s_add_c.enable(&mut region, 2)?;
                let x_a6 = region.assign_advice(|| "x_a6",
                    config.advice[0].clone(), 2, || t5)?;
                region.assign_fixed(|| "constant 5",
                    config.constant.clone(), 2, || Value::known((F::from(5))))?;
                let x_c6 = region.assign_advice(|| "x_c6",
                    config.advice[2].clone(), 2, || t6)?;
                Ok((
                    (x_a4.cell(), x_b4.cell(), x_c4.cell()),
                    (x_a5.cell(), x_b5.cell(), x_c5.cell()),
                    (x_a6.cell(), x_c6.cell())
                ))
            }
        )?;

        // t6 is result, assign instance
        layouter.constrain_instance(x_c6, config.instance, 0)?;

        // enforce copy constraints
        layouter.assign_region(|| "equality",
            |mut region| {
                region.constrain_equal(x_a1, x_a2)?; // namely, x_a1 = x_a2
                region.constrain_equal(x_a2, x_b1)?; // namely, x_a2 = x_b1

                region.constrain_equal(x_b2, x_b5)?; // namely, x_b2 = x_b5

                region.constrain_equal(x_a4, x_c1)?; // namely, x_a4 = x_c1

                region.constrain_equal(x_a3, x_c2)?; // namely, x_a3 = x_c2

                region.constrain_equal(x_b4, x_c3)?; // namely, x_b4 = x_c3

                region.constrain_equal(x_a5, x_c4)?; // namely, x_a5 = x_c4

                region.constrain_equal(x_a6, x_c5)?; // namely, x_a6 = x_c5
                Ok(())
            }
        )?;
        Ok(())
    }
}

fn main() {
    use halo2_proofs::{dev::MockProver, pasta::Fp};

    // ANCHOR: test-circuit
    // The number of rows in our circuit cannot exceed 2^k. Since our example
    // circuit is very small, we can pick a very small value here.
    let k = 4;

    // Prepare the private and public inputs to the circuit!
    let constant = Fp::from(7);
    let a = Fp::from(2);
    let b = Fp::from(3);
    let c = a * b;

    // Instantiate the circuit with the private inputs.
    let circuit = MyCircuit {
        u: Value::known(a),
        v: Value::known(b),
    };

    // Arrange the public input. We expose the multiplication result in row 0
    // of the instance column, so we position it there in our public inputs.
    let public_inputs = vec![c];
    println!("public inputs: {:?}", public_inputs);

    // Given the correct public input, our circuit will verify.
    let prover = MockProver::run(k, &circuit, vec![public_inputs.clone()]).unwrap();
    assert_eq!(prover.verify(), Ok(()));

    // If we try some other public input, the proof will fail!
    // public_inputs[0] += Fp::one();
    // let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
    // assert!(prover.verify().is_err());
    //println!("public inputs: {:?}", public_inputs[0]);
    // ANCHOR_END: test-circuit
}