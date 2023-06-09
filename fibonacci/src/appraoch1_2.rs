//Difference: if you want two chips to reuse the same columns, you have to manually specift them
use std::marker::PhantomData;

use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::*,
    plonk::*, poly::Rotation,
    pasta::Fp, dev::MockProver,
};

#[derive(Debug, Clone)]
struct ACell<F: FieldExt>(AssignedCell<F, F>);

#[derive(Debug, Clone)]
struct FiboConfig{
    pub advice: [Column<Advice>; 3],
    pub selector: Selector,
    pub instance: Column<Instance>,
} 

struct FiboChip<F: FieldExt>{
    config: FiboConfig,
    _marker: PhantomData<F>,
}

impl<F:FieldExt> FiboChip<F>  {
    fn construct(config: FiboConfig) -> Self {
        Self { config, _marker: PhantomData}
    }

    fn configure(meta: &mut ConstraintSystem<F>, advice: [Column<Advice>; 3], instance: Column<Instance>) -> FiboConfig {
   
        let col_a= advice[0];
        let col_b= advice[1];
        let col_c= advice[2];
        let selector: Selector= meta.selector();

        meta.enable_equality(col_a);
        meta.enable_equality(col_b);
        meta.enable_equality(col_c);
        meta.enable_equality(instance);


        meta.create_gate("add", |meta|{
            //this expression will usually correspond to a cell like a relative cell inside a custom gate
            let s= meta.query_selector(selector);
            let a= meta.query_advice(col_a, Rotation::cur());
            let b= meta.query_advice(col_b, Rotation::cur());
            let c= meta.query_advice(col_c, Rotation::cur());
            //Rotation::next(): you query the next row, relative next row for this cell
            //With Rotation, we can define an offset like 5, 20, -100, etc. It is relative to the row.
            vec![s*(a + b - c)] // means s * ( a + b - c) == 0
        });
        FiboConfig { 
            advice: [col_a, col_b, col_c ], 
            selector, 
            instance,
        }
    }

    fn assign_first_row(&self, mut layouter: impl Layouter<F>, a: Option<F>, b: Option<F>) -> Result
    <(ACell<F>, ACell<F>, ACell<F>), Error>{
        layouter.assign_region(||"first row", |mut region|{
            self.config.selector.enable(&mut region, 0)?;

            let a_cell= region.assign_advice(
                || "a",
                self.config.advice[0],
                0,
                || a.ok_or(Error::Synthesis),
            ).map(ACell)?;

            let b_cell= region.assign_advice(
                || "b",
                self.config.advice[1],
                0,
                || b.ok_or(Error::Synthesis),
            ).map(ACell)?;

            let c_val= a.and_then(|a| b.map(|b| a + b));

            let c_cell= region.assign_advice(
                || "c",
                self.config.advice[2],
                0,
                || c_val.ok_or(Error::Synthesis),
            ).map(ACell)?;

            Ok((a_cell, b_cell, c_cell))

        })
    }

    fn assign_row(&self, mut layouter: impl Layouter<F>, prev_b: &ACell<F>, prev_c: &ACell<F>) -> Result<ACell<F>, Error> {
        layouter.assign_region(||"next row", |mut region|{
            self.config.selector.enable(&mut region, 0)?;//enable the selector to turn on the custom gate

            prev_b.0.copy_advice(||"a", &mut region, self.config.advice[0], 0)?;
            //prev_b.0.copy_advice(||"a", &mut region: current region, self.config.advice[0]: the first advice column inside our config(row), 0: offset like the current row, the first row in the region)?; a description of the description previous line
            prev_c.0.copy_advice(||"b", &mut region, self.config.advice[1], 0)?;

            let c_val= prev_b.0.value().and_then(|b| {
                prev_c.0.value().map(|c| *b + *c)
            });

            let c_cell= region.assign_advice(||"c", self.config.advice[2], 0, ||c_val.ok_or(Error::Synthesis)).map(ACell)?;
            Ok(c_cell)
        })
    }

    //We will take an assigned cell and then constrain to be equal the instance column value
    pub fn expose_public(&self, mut layouter: impl Layouter<F>, cell: &ACell<F>, row: usize/*an absolute row number inside the instance column*/) -> Result<(), Error>{
        layouter.constrain_instance(cell.0.cell(), self.config.instance, row)
    }
}

#[derive(Default)]
struct MyCircuit<F>{
    pub a: Option<F>,
    pub b: Option<F>,
}

impl<F:FieldExt> Circuit<F> for MyCircuit<F> {
    type Config = FiboConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let col_a= meta.advice_column();
        let col_b= meta.advice_column();
        let col_c= meta.advice_column();
        let instance= meta.instance_column();

        FiboChip::configure(meta, [col_a, col_b, col_c], instance)
    }

    fn synthesize(&self, config: Self::Config, mut layouter: impl Layouter<F>) -> Result<(), Error> {
        let chip= FiboChip::construct(config);

        let (prev_a, mut prev_b, mut prev_c)= chip.assign_first_row(layouter.namespace(||"first row"), self.a, self.b)?;
        
        chip.expose_public(layouter.namespace(||"private a"), &prev_a, 0);
        chip.expose_public(layouter.namespace(||"private b"), &prev_b, 1);

        for _i in 3..10 {
            let c_cell= chip. assign_row(layouter.namespace(||"next row"), &prev_b, &prev_c)?;
            prev_b= prev_c;
            prev_c= c_cell;
        }

        chip.expose_public(layouter.namespace(||"out"), &prev_c, 2);

        Ok(())
    }
}

fn main(){
    //instantiate a circuit

    let k= 4;//the size of the circuit

    let a= Fp::from(1);
    let b= Fp::from(1);
    let out= Fp::from(55);

    let circuit= MyCircuit{
        a: Some(a),
        b: Some(b),
    };

    let mut public_input= vec![a, b, out];

    let prover= MockProver::run(k, &circuit, vec![public_input.clone()]).unwrap();
    prover.assert_satisfied();

    public_input[2] += Fp::one();

    let prover= MockProver::run(k, &circuit, vec![public_input.clone()]).unwrap();
    prover.assert_satisfied()
}