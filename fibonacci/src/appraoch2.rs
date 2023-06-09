use std::marker::PhantomData;
use halo2_proofs::{arithmetic::FieldExt, circuit::*, plonk::*, poly::Rotation, pasta::Fp, dev::MockProver,};

#[derive(Debug, Clone)]
struct ACell<F: FieldExt>(AssignedCell<F, F>);

#[derive(Debug, Clone)]
struct FibonacciConfig {
    pub advice: Column<Advice>,
    pub selector: Selector,
    pub instance: Column<Instance>,
}

#[derive(Debug, Clone)]
struct FibonacciChip<F: FieldExt> {
    config: FibonacciConfig,
    _marker: PhantomData<F>,
}

impl<F: FieldExt> FibonacciChip<F> {
    pub fn construct(config: FibonacciConfig) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }

    pub fn configure(meta: &mut ConstraintSystem<F>,advice: Column<Advice>, instance: Column<Instance>) -> FibonacciConfig {
        let selector = meta.selector();

        meta.enable_equality(advice);
        meta.enable_equality(instance);

        meta.create_gate("add", |meta| {
            //
            // advice  | selector
            //   a    |     s
            //   b    |
            //   c    |
            let s = meta.query_selector(selector);
            let a = meta.query_advice(advice, Rotation::cur());
            let b = meta.query_advice(advice, Rotation::next());
            let c = meta.query_advice(advice, Rotation(2));
            vec![s * (a + b - c)]
        });

        FibonacciConfig {
            advice,
            selector,
            instance,
        }
    }

    fn assign(&self, mut layouter: impl Layouter<F>, nrows:usize) -> Result
    <AssignedCell<F, F>, Error>{
        layouter.assign_region(
            || "entire fibonacci table",
            |mut region| {
                self.config.selector.enable(&mut region, 0)?;
                self.config.selector.enable(&mut region, 1)?;

                let mut a_cell= region.assign_advice_from_instance(||"1", self.config.instance, 0, self.config.advice, 0)?;
                let mut b_cell= region.assign_advice_from_instance(||"2", self.config.instance, 1, self.config.advice, 1)?;

                for row in 2..nrows{
                    if row < nrows - 2{
                        self.config.selector.enable(&mut region, row)?;
                    }

                    let c_val= a_cell.value().and_then(|a|{
                        b_cell.value().map(|b| *a + *b)
                    });
                    let c_cell= region.assign_advice(||"advice", self.config.advice, row, ||c_val.ok_or(Error::Synthesis))?;
                    
                    a_cell= b_cell;
                    b_cell= c_cell;
                }

                Ok(b_cell)
            },
        )
    }

    pub fn expose_public(
        &self,
        mut layouter: impl Layouter<F>,
        cell: AssignedCell<F, F>,
        row: usize,
    ) -> Result<(), Error> {
        layouter.constrain_instance(cell.cell(), self.config.instance, row)
    }
}

#[derive(Default)]
struct MyCircuit<F>{
    pub a: Option<F>,
    pub b: Option<F>,
}

impl<F: FieldExt> Circuit<F> for MyCircuit<F> {
    type Config = FibonacciConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let advice= meta.advice_column();
        let instance= meta.instance_column();
        FibonacciChip::configure(meta, advice, instance)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        let chip = FibonacciChip::construct(config);

        let out_cell= chip.assign(layouter.namespace(||"entire table"), 10)?;

        chip.expose_public(layouter.namespace(|| "out"), out_cell, 2)?;

        Ok(())
    }
}


fn main(){

    let k= 4;//the size of the circuit

    let a = Fp::from(1); // F[0]
    let b = Fp::from(1); // F[1]
    let out = Fp::from(55); // F[9]

    let circuit= MyCircuit{
        a: Some(a),
        b: Some(b),
    };

    let mut public_input = vec![a, b, out];

    let prover = MockProver::run(k, &circuit, vec![public_input.clone()]).unwrap();
    prover.assert_satisfied();

    public_input[2] += Fp::one();
    let _prover = MockProver::run(k, &circuit, vec![public_input]).unwrap();
    _prover.assert_satisfied();

}

/*mod tests {
    use super::*;
    #[cfg(feature = "dev-graph")]
    #[test]
    fn print(){

    let k = 4;
    let a = Fp::from(1);
    let b = Fp::from(1);
    let out = Fp::from(55);
    let circuit = FibonacciCircuit {
        a:Some(a), b:Some(b)
    };
    let mut public_inputs = vec![a, b, out];
    // This prover is faster and 'fake', but is mostly a devtool for debugging
    let prover = MockProver::run(k, &circuit, vec![public_inputs.clone()]).unwrap();
    // This function will pretty-print on errors
    prover.assert_satisfied();
    public_inputs[2] += Fp::one();
    let prover = MockProver::run(k, &circuit, vec![public_inputs.clone()]).unwrap();
    // prover.assert_satisfied();

    use plotters::prelude::*;

    let root = BitMapBackend::new("fib-2—layout.png", (1024, 7680)).into_drawing_area();
    //root.fiti(&WHITE).unwrap();
    let root1 = root.titled("Fib 2 Layout", ("sans—serif", 60)).unwrap();
    let circuit:FibonacciCircuit<Fp> = FibonacciCircuit { a: None, b: None };
    halo2_proofs::dev::CircuitLayout::default()
        .render(4, &circuit, &root1)
        .unwrap();

    }
}*/

#[cfg(test)]
mod tests {
    use super::MyCircuit;
    use std::marker::PhantomData;
    use halo2_proofs::{dev::MockProver, pasta::Fp};

    #[test]
    fn fibonacci_example2() {
        let k = 4;

        let a = Fp::from(1); // F[0]
        let b = Fp::from(1); // F[1]
        let out = Fp::from(55); // F[9]

        let circuit = MyCircuit{
            a: Some(a),
            b: Some(b),
        };

        let mut public_input = vec![a, b, out];

        let prover = MockProver::run(k, &circuit, vec![public_input.clone()]).unwrap();
        prover.assert_satisfied();

        public_input[2] += Fp::one();
        let _prover = MockProver::run(k, &circuit, vec![public_input]).unwrap();
        // uncomment the following line and the assert will fail
        // _prover.assert_satisfied();
    }

    #[cfg(feature = "dev-graph")]
    #[test]
    fn plot_fibo2() {
        use plotters::prelude::*;
        let root = BitMapBackend::new("fib-2-layout.png", (1024, 3096)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let root = root.titled("Fib 2 Layout", ("sans-serif", 60)).unwrap();

        let a = Fp::from(1); // F[0]
        let b = Fp::from(1); // F[1]
        
        let circuit:MyCircuit<Fp> = MyCircuit { a: None, b: None };
        halo2_proofs::dev::CircuitLayout::default()
            .render(4, &circuit, &root)
            .unwrap();
    }
}