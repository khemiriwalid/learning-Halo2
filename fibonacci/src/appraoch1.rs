use std::marker::PhantomData;

use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::*,
    plonk::*, poly::Rotation,
};

#[derive(Debug, Clone)]
struct FiboConfig{
    pub advice: [Column<Advice>; 3],
    pub selector: Selector,
} 

struct FiboChip<F: FieldExt>{
    config: FiboConfig,
    _marker: PhantomData<F>,
}

impl<F:FieldExt> FiboChip<F>  {
    fn construct(config: FiboConfig) -> Self {
        Self { config, _marker: PhantomData}
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> FiboConfig {
        let col_a= meta.advice_column();
        let col_b= meta.advice_column();
        let col_c= meta.advice_column();
        let selector: Selector= meta.selector();

        meta.enable_equality(col_a);
        meta.enable_equality(col_b);
        meta.enable_equality(col_c);


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
        }
    }
}

#[derive(Default)]
struct MyCircuit<F>{
    pub a: Option<F>,
    pub b: Option<F>,
}

/*impl<F:FieldExt> Circuit<F> for MyCircuit<F> {
    type Config = FiboConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        FiboChip::configure(meta)
    }

    fn synthesize(&self, config: Self::Config, layouter: impl Layouter<F>) -> Result<(), Error> {
        
    }
}*/

fn main(){

}