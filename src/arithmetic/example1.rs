use std::marker::PhantomData;

use halo2_proofs::{
    arithmetic::FieldExt, circuit::*, dev::MockProver, pasta::Fp, plonk::*, poly::Rotation
};

#[derive(Debug, Clone)]
struct ACell<F: FieldExt>(AssignedCell<F, F>);

#[derive(Debug, Clone)]
struct ArithmeticConfig {
    advice: [Column<Advice>; 3],
    instance: Column<Instance>,
    s_add: Selector,
    s_mul: Selector,
}

struct ArithmeticChip<F: FieldExt> {
    config: ArithmeticConfig,
    _marker: PhantomData<F>,
}

impl<F: FieldExt>  ArithmeticChip<F>{
    pub fn construct(config: ArithmeticConfig) -> Self {
        Self {config, _marker: PhantomData }
    }

    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        advice: [Column<Advice>; 3],
        instance: Column<Instance>,
    ) -> ArithmeticConfig {
        // enable equality to check the permutation on the specified columns
        for column in &advice{
            meta.enable_equality(*column);
        }

        meta.enable_equality(instance);
        
        let s_add = meta.selector();
        let s_mul = meta.selector();
       // let s_add_c = meta.selector();
       // let s_mul_c = meta.selector();

        meta.create_gate("add", |meta|{
            let s_add = meta.query_selector(s_add);
            let lhs = meta.query_advice(advice[0], Rotation::cur());
            let rhs = meta.query_advice(advice[1], Rotation::cur());
            let out = meta.query_advice(advice[2], Rotation::cur());

            vec![s_add * (lhs + rhs - out)]
        });

        meta.create_gate("mul", |meta|{
            let s_mul = meta.query_selector(s_mul);
            let lhs = meta.query_advice(advice[0], Rotation::cur());
            let rhs = meta.query_advice(advice[1], Rotation::cur());
            let out = meta.query_advice(advice[2], Rotation::cur());

            vec![s_mul * (lhs * rhs - out)]
        });

        // meta.create_gate("add_with_const", |meta|{
        //     let s_add_c = meta.query_selector(s_add_c);
        //     let lhs = meta.query_advice(advice[0], Rotation::cur());
        //     let rhs  = meta.query_fixed(constant, Rotation::cur());
        //     let out = meta.query_advice(advice[2], Rotation::cur());

        //     vec![s_add_c * (lhs + rhs - out)]
        // });

        ArithmeticConfig {
            advice,
            instance,
            s_add,
            s_mul,      
        }

    }

    pub fn assign_add (
        &self,
        mut layouter: impl  Layouter<F>,
        a: Value<F>,
        b: Value<F>,
    ) -> Result<(ACell<F>, ACell<F>, ACell<F>), Error> {
        layouter.assign_region(|| "add", 
            |mut region|{
                self.config.s_add.enable(&mut region, 0)?;
                let a_cell = region.assign_advice(
                       || "a", 
                       self.config.advice[0], 
                       0, 
                       || a,
            ).map(ACell)?;


                let b_cell = region.assign_advice(
                    || "b", 
                    self.config.advice[1],
                    0, 
                    || b,
                ).map(ACell)?;
    

                let c_val = a.and_then(|a| b.map(|b| a+b));
                let c_cell = region.assign_advice(
                    || "b", 
                    self.config.advice[2],
                    0, 
                    || c_val,
                ).map(ACell)?;
                 
        Ok((a_cell, b_cell, c_cell))
    })        
    }

    pub fn assign_mul (
        &self,
        mut layouter: impl  Layouter<F>,
        a: Value<F>,
        b: Value<F>,
    ) -> Result<(ACell<F>, ACell<F>, ACell<F>), Error> {
        layouter.assign_region(|| "mul", 
            |mut region|{
                self.config.s_mul.enable(&mut region, 0)?;
                let a_cell = region.assign_advice(
                       || "a", 
                       self.config.advice[0], 
                       0, 
                       || a,
                    ).map(ACell)?;


                let b_cell = region.assign_advice(
                    || "b", 
                    self.config.advice[1],
                    0, 
                    || b,
                    ).map(ACell)?;
    

                let c_val = a.and_then(|a| b.map(|b| a*b));
                let c_cell = region.assign_advice(
                    || "b", 
                    self.config.advice[2],
                    0, 
                    || c_val,
                ).map(ACell)?;
                 
        Ok((a_cell, b_cell, c_cell))
    })        
    }


    pub fn expose_public(
        &self,
        mut layouter: impl  Layouter<F>,
        cell: &ACell<F>,
        row: usize,
    )-> Result<(), Error>{
        layouter.constrain_instance(cell.0.cell(), self.config.instance, row)

    }
}

#[derive(Default)]
struct ArithmeticCircuit<F> {
    a: Value<F>,
    b: Value<F>
}

impl<F: FieldExt> Circuit<F> for ArithmeticCircuit<F> {
    type Config = ArithmeticConfig;

    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let col_a = meta.advice_column();
        let col_b = meta.advice_column();
        let col_c = meta.advice_column();
        let instance = meta.instance_column();
       // let constant = meta.fixed_column();

        ArithmeticChip::configure(meta, 
                [col_a, col_b, col_c], instance,
        )}

    fn synthesize(&self, config: Self::Config, mut layouter: impl Layouter<F>) -> Result<(), Error> {
        let chip = ArithmeticChip::construct(config);

        let (a_0, _b_0, c_0) = chip.assign_add(
            layouter.namespace(|| "add"), 
            self.a, 
            self.b
         )?;

        let (_a_1, _b_1, _c_1) = chip.assign_mul(
            layouter.namespace(|| "mul"), 
            a_0.0.value().map(|v1| *v1), 
            c_0.0.value().map(|v2| *v2),
        )?;

        let _ = chip.expose_public(layouter.namespace(|| "out"), &_c_1, 0);
        
//         layouter.assign_region(|| "equality",
//             |mut region| {
//                 region.constrain_equal(a_0.0.cell(), a_1.0.cell())?; // namely, a_0 = a_1
//             }
         Ok(())
  }
}

fn main() {
    let k = 4;

    let a: Fp = Fp::from(2);
    let b: Fp = Fp::from(1);
    let c: Fp = a + b;

    let out = a * c;

    let circuit = ArithmeticCircuit {
        a: Value::known(a),
        b: Value::known(b)
    };

    let mut  _public_input = vec![out];
    

    let prover = MockProver::run(k, &circuit, vec![_public_input.clone()]).unwrap();
    prover.assert_satisfied();

    println!("c = {:?}", out);
    // _public_input[2] += Fp::one();
    // let prover = MockProver::run(k, &circuit, vec![_public_input.clone()]).unwrap();
    // prover.assert_satisfied();

}
