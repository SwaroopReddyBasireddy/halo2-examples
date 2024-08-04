use gadget::is_zero::{IsZeroChip, IsZeroConfig};

use halo2_proofs::{
    arithmetic::FieldExt, circuit::*, dev::MockProver, pasta::Fp, plonk::*, poly::Rotation
};


#[derive(Debug,Clone)]
struct FunctionConfig<F: FieldExt> {
    selector: Selector,
    a: Column<Advice>,
    b: Column<Advice>,
    c: Column<Advice>,
    output: Column<Advice>,
    a_equals_b: IsZeroConfig<F>,
}

struct FunctionChip<F: FieldExt> {
    config: FunctionConfig<F>,
}

impl <F: FieldExt> FunctionChip<F> {
    pub fn construct (config: FunctionConfig<F>) -> Self {
        FunctionChip {config}
    }

    pub fn configure(meta: &mut ConstraintSystem<F>) -> FunctionConfig<F>{
        let selector = meta.selector();
        let a = meta.advice_column();
        let b = meta.advice_column();
        let c = meta.advice_column();
        let output = meta.advice_column();

        let is_zero_advice_column = meta.advice_column();
        let a_equals_b = IsZeroChip::configure(
                      meta, 
                      |meta| meta.query_selector(selector), 
                      |meta| meta.query_advice(a, Rotation::cur()) - meta.query_advice(b, Rotation::cur()), 
                      is_zero_advice_column,
                    );

        meta.create_gate("f(a, b, c) = if a == b {c} else {a-b}", |meta|{
            let s = meta.query_selector(selector);
            let a = meta.query_advice(a, Rotation::cur());
            let b = meta.query_advice(b, Rotation::cur());
            let c = meta.query_advice(c, Rotation::cur());
            let output = meta.query_advice(output, Rotation::cur());
    
        vec![
            s.clone() * (a_equals_b.expr() * (output.clone() - c)),
            s * (Expression::Constant(F::one())- a_equals_b.expr()) * (output - (a-b))
        ]

    });       

    FunctionConfig {
        selector,
        a,
        b,
        c,
        output,
        a_equals_b,
        }
    }

    pub fn assign(
        &self,
        mut layouter: impl Layouter<F>,
        a: F,
        b: F,
        c: F,
    ) -> Result<(), Error> {

        let is_zero_chip = IsZeroChip::construct(self.config.a_equals_b.clone());

        let _ = layouter.assign_region(
            || "f(a,b,c) = if a == b {c} else {a-b}",
            |mut region| Ok({
                self.config.selector.enable(&mut region, 0)?;
                region.assign_advice(|| "a", self.config.a, 0, || Value::known(a))?;
                region.assign_advice(|| "b", self.config.b, 0, || Value::known(b))?;
                region.assign_advice(|| "c", self.config.c, 0, || Value::known(c))?;

                is_zero_chip.assign(&mut region, 0, Value::known(a-b))?;

                let output = if a == b {c} else { a-b };
                region.assign_advice(|| "output", self.config.output, 0, || Value::known(output))?;
        }),
     );

     Ok(())
    }
}

#[derive(Default)]
struct FunctionCircuit<F> {
    a: F,
    b: F,
    c: F,
}

impl<F: FieldExt> Circuit<F> for FunctionCircuit<F>  {
    type Config = FunctionConfig<F>;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        FunctionChip::configure(meta)
    }

  
    fn synthesize(&self, config: Self::Config, layouter: impl Layouter<F>) -> Result<(), Error> {
        let chip = FunctionChip::construct(config);
        chip.assign(layouter, self.a, self.b, self.c)?;

        Ok(())
    }
}

fn main() {
    let circuit = FunctionCircuit {
        a: Fp::from(10),
        b: Fp::from(20),
        c: Fp::from(15),
    };
    let prover = MockProver::run(4, &circuit, vec![]).unwrap();
    prover.assert_satisfied();

    println!("Hello World");

}


#[cfg(test)]
mod tests {
    use halo2_proofs::pasta::Fp;
    use super::*;

    #[test]
    fn iszero_example() {
        let k = 4;

        let circuit = FunctionCircuit {
            a: Fp::from(10),
            b: Fp::from(20),
            c: Fp::from(15),
        };
        let prover = MockProver::run(k, &circuit, vec![]).unwrap();
        prover.assert_satisfied();
    
        println!("Hello World");
    
    }

    #[cfg(feature = "dev-graph")]
    #[test]
    fn plot_iszerfunction() {
        use plotters::prelude::*;

        let root = BitMapBackend::new("iszero -1-layout.png", (1024, 3096)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let root = root.titled("isZero-Function 1 Layout", ("sans-serif", 60)).unwrap();

        let circuit = FunctionCircuit {
            a: Fp::from(10),
            b: Fp::from(20),
            c: Fp::from(15),
        };
        halo2_proofs::dev::CircuitLayout::default()
            .render(4, &circuit, &root)
            .unwrap();
    }
}