use std::marker::PhantomData;

use halo2_proofs::{
    arithmetic::FieldExt, circuit::*, dev::MockProver, pasta::Fp, plonk::*, poly::Rotation
};

#[derive(Debug, Clone)]
struct ACell<F: FieldExt>(AssignedCell<F, F>);

#[derive(Debug, Clone)]
struct FiboConfig {
    pub advice: Column<Advice>,
    pub selector: Selector,
    pub instance: Column<Instance>,
}

#[derive(Debug, Clone)]
struct FiboChip<F: FieldExt> {
    config: FiboConfig,
    _marker: PhantomData<F>
}

impl<F: FieldExt> FiboChip<F>  {
    fn construct(config: FiboConfig) -> Self{
        Self {config, _marker: PhantomData}
    }  

    // Define columns or custom gates
    fn configure(
        meta: &mut ConstraintSystem<F>, 
        advice: Column<Advice>,
        instance: Column<Instance>
    ) -> FiboConfig {
        let selector = meta.selector();
        
        // enable equality to check the permutation on the specified columns
        meta.enable_equality(advice);
        
        meta.enable_equality(instance);

        // Define our add gate
        meta.create_gate("add", |meta| {
            //
            //  advice   |  selector
            //      a    |     s
            //      b    |
            //      c    | 

            let s = meta.query_selector(selector);
            let a = meta.query_advice(advice, Rotation::cur());
            let b = meta.query_advice(advice, Rotation::next());
            let c = meta.query_advice(advice, Rotation(2));
            vec![s*(a+b-c)] // constraint to be return by the custom gate "add"
        });

        FiboConfig {
            advice,
            selector,
            instance,
        }
    } 

    fn assign(
        &self, 
        mut layouter: impl  Layouter<F>, 
        nrows: usize
    ) ->  Result<AssignedCell<F, F>, Error> {
        layouter.assign_region(|| "entire table", 
        |mut region| {
            self.config.selector.enable(&mut region, 0)?;
            self.config.selector.enable(&mut region, 1)?;
            
            let mut a_cell = region.assign_advice_from_instance(
                
                || "1", 
                self.config.instance, 
                0, 
                self.config.advice, 
                0
            )?;

            let mut b_cell = region.assign_advice_from_instance(
                
                || "1", 
                self.config.instance, 
                1, 
                self.config.advice, 
                1
            )?;

            for row in 2..nrows {
                if row < nrows - 2 {
                    let _ = self.config.selector.enable(&mut region, row);
                }
                let c_val = a_cell.value().and_then(
                    |a| {
                        b_cell.value().map(|b| *a + *b)
                    });
                let c_cell = region.assign_advice(
                    || "advice", 
                    self.config.advice,
                    row, 
                   // || a_cell.value() + b_cell.value(),
                    || c_val.ok_or(Error::Synthesis),
                    )?;

                    a_cell = b_cell;
                    b_cell = c_cell;
           
            }

            Ok(b_cell)     
        },
    )
    }

    
    pub fn expose_public(
        &self,
        mut layouter: impl  Layouter<F>,
        cell: AssignedCell<F, F>,
        row: usize,
    )-> Result<(), Error>{
        layouter.constrain_instance(cell.cell(), self.config.instance, row)

    }
 
}

#[derive(Default)]
struct MyCircuit<F>(PhantomData<F>);

impl <F: FieldExt> Circuit<F> for MyCircuit<F> {
    type Config = FiboConfig;

    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let advice = meta.advice_column();
        let instance = meta.instance_column();

        FiboChip::configure(meta, advice, instance)
    }

    fn synthesize(&self, 
        config: Self::Config, 
        mut layouter: impl Layouter<F>
    ) -> Result<(), Error> {
        let chip = FiboChip::construct(config);

        let out_cell: AssignedCell<F, F> = chip.assign(
            layouter.namespace(|| "entire table"),
             10
            )?;

        chip.expose_public(layouter.namespace(|| "out_cell"), out_cell, 2)?;

        Ok(())
    }

    
}

fn main () {
    
    let k = 4;

    let a: Fp = Fp::from(1);
    let b: Fp = Fp::from(1);
    let out: Fp = Fp::from(55);

    let circuit = MyCircuit::<Fp>(PhantomData);
        
    let mut  _public_input = vec![a, b, out];

    let prover = MockProver::run(k, &circuit, vec![_public_input.clone()]).unwrap();
    prover.assert_satisfied();
    
    _public_input[2] += Fp::one();
    let prover = MockProver::run(k, &circuit, vec![_public_input]).unwrap();
    prover.assert_satisfied();


    // use halo2_proofs::pasta::Fp;
    // use plotters::prelude::*;
    
    // let root = BitMapBackend::new("fib2-layout.png", (1024, 7680)).into_drawing_area();
    // root.fill(&WHITE).unwrap();
    // let root = root.titled("fib-2 Layout", ("sans-serif")).unwrap();

    // let circuit = MyCircuit::<Fp>(PhantomData);
        

    // halo2_proofs::dev::CircuitLayout::default()
    // .render(4, &circuit, &root)
    // .unwrap();
}

