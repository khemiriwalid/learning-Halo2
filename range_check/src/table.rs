use std::marker::PhantomData;
use halo2_proofs::{plonk::{TableColumn, Error, ConstraintSystem}, arithmetic::FieldExt, circuit::{Value, Layouter}};
// a lookup table of values up to RANGE.
//e.g. RANGE= 256, values= [0..255]

#[derive(Debug, Clone)]
pub struct RangeCheckTable<F: FieldExt, const RANGE: usize>{
    pub value: TableColumn,
    _marker: PhantomData<F>
}
//We want to implement a load function: it assigns all the fixed values to the table(like other fixed column,at key gen time)
impl<F: FieldExt, const RANGE: usize> RangeCheckTable<F, RANGE>{

    pub fn configure(meta: &mut ConstraintSystem<F>) -> Self {
        let value= meta.lookup_table_column();
        Self { value, _marker: PhantomData }
    }

    pub fn load(&self, layouter: &mut impl Layouter<F>) -> Result<(), Error>{
        // a special API for lookup table
        //it is like assign region except like bespoke and only works for tables(it is about making lookup tables safe)
        layouter.assign_table(||"load ranhe-check table", |mut table|{
            let mut offset= 0;
            for i in 0..RANGE{
                table.assign_cell(||"assign cell", self.value, offset, ||Value::known(F::from(i as u64)))?;
                offset+= 1;
            }
            Ok(())
        })
    }
}