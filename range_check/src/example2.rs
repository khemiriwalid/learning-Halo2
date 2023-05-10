// This helper checks that the value witnessed in a given cell is within a given range.
//layout: an advice column where you witness a value and a selector where you enable the range check constraint.
/*Depending on the range, this helper uses either a range-check expression(for small ranges)
    or lookup (for larger ranges)
    If we have a very large R, then the polynomial is going to be very hugh degree and that will increase
    the cost of the circuit
*/
//     value  | q_range_check | q_lookup | table_value
// ----------------------------------------------------
//      v     |       1       |     0     |     0
//      v'    |       0       |     1     |     1
// When writing configs, it's best practice to pass in advice columns beacause advice columns are very often shared across configs. 
use halo2_proofs::{
    plonk::*,
    circuit::{AssignedCell, Layouter, Value},
    arithmetic::FieldExt, poly::Rotation,
};
use std::marker::PhantomData;
mod table;
use table::RangeCheckTable;


#[derive(Debug, Clone)]
struct RangeCheckConfig<F: FieldExt, const RANGE: usize, const LOOKUP_RANGE: usize>{
    value: Column<Advice>,
    q_range_check: Selector,
    q_lookup: Selector,
    table: RangeCheckTable<F, LOOKUP_RANGE>
}

impl<F: FieldExt, const RANGE: usize, const LOOKUP_RANGE: usize> RangeCheckConfig<F, RANGE, LOOKUP_RANGE>{
    fn configure(meta: &mut ConstraintSystem<F>, value: Column<Advice>) -> Self{
        //Toggles the range check constraint
        let q_range_check= meta.selector();

        //Toggles the lookup argument
        let q_lookup= meta.complex_selector();

        // Configure a lookup table
        let table= RangeCheckTable::configure(meta);

        let config= Self{
            q_range_check,
            value,
            table: table.clone(),
            q_lookup
        };

        /* 
            A single gate can have multiple constraints all toggled by the same selector. When you
            have multiple constraints, it's best practice to name them.
        */

        // Range-check gate
        // For  a value v and a range R, check that v < R
        //v * (1 - v) * (2 - v) * ... * (R - 1 - v) = 0
        /*notice: when we query a selector, we don't specify the rotation becasue by definition a 
        //selector is always query at the current rotation and the advice columns that create relative
        to the selectors offset*/
        meta.create_gate("Range check", |meta|{
            let q_range_check= meta.query_selector(q_range_check);
            let value= meta.query_advice(value, Rotation::cur());

            let range_check= |range: usize, value: Expression<F>|{
                (0..range).fold(value.clone(), |expr, i|{
                    expr * (Expression::Constant(F::from(i as u64)) - value.clone())
                })
            };
            /*
                Previously, we just returned a vector of expressions at the end of create_gate,
                 Constraints::with_selector is doing the same thing. However, it's kind of 
                 abstracting the selector away from you. So, you specify one selector and then behind
                 the scenes it multiplies each expression by that selector. It is a cleaner way to do the
                 same thing.
             */
            Constraints::with_selector(q_range_check, [("range_check", range_check(RANGE, value))])
        });

        //Range check lookup
        //Check that a value v is contained within a lookup table of values 0..RANGE
        //that's our lookup argument that we have to configure at key gen time
        meta.lookup(|meta|{
            let q_lookup= meta.query_selector(q_lookup);
            let value= meta.query_advice(value, Rotation::cur());
            vec![(q_lookup * value, table.value)]
        });

        config
    }

    /*
    How can we make the configure and assign APIs better(well) connected?
    They are pretty disjoint. We have to more or less remember the shape in which we configured
    things and manually amke sure that we assign things in that exact shape. That's a lot of overhed
    for the developer
    */
    fn assign(&self, mut layouter: impl Layouter<F>, value: Value<Assigned<F>>, range: usize) -> Result<(), Error>{
        assert!(range <= RANGE);
        if(range < RANGE) {
            layouter.assign_region(||"Assign value", |mut region|{
                let offset= 0;
                // Enable q_range_check
                self.q_range_check.enable(&mut region, offset)?;
    
                //Assign given value
                region.assign_advice(||"assign value", self.value, offset, ||value)?;
                Ok(())
            })
        }else {
            layouter.assign_region(||"Assign value for lookup range check", |mut region|{
                let offset= 0;
                // Enable q_lookup
                self.q_lookup.enable(&mut region, offset)?;
                    
                //Assign given value
                region.assign_advice(||"assign value", self.value, offset, ||value)?;
                Ok(())
            })
        }
      
    }
}

#[cfg(test)]
mod tests {
    use halo2_proofs::{
        circuit::floor_planner::V1,
        dev::{FailureLocation, MockProver, VerifyFailure},
        pasta::Fp,
        plonk::{Any, Circuit},
    };

    use super::*;

    #[derive(Default)]
    struct MyCircuit<F: FieldExt, const RANGE: usize, const LOOKUP_RANGE: usize> {
        value: Value<Assigned<F>>,
        large_value: Value<Assigned<F>>,
    }

    impl<F: FieldExt, const RANGE: usize, const LOOKUP_RANGE: usize> Circuit<F> for MyCircuit<F, RANGE,LOOKUP_RANGE> {
        type Config = RangeCheckConfig<F, RANGE, LOOKUP_RANGE>;
        type FloorPlanner = V1;

        fn without_witnesses(&self) -> Self {
            Self::default()
        }

        fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
            let value = meta.advice_column();
            RangeCheckConfig::configure(meta, value)
        }

        fn synthesize(
            &self,
            config: Self::Config,
            mut layouter: impl Layouter<F>,
        ) -> Result<(), Error> {
            config.table.load(&mut layouter)?;
            config.assign(layouter.namespace(|| "Assign value"), self.value, RANGE)?;
            config.assign(layouter.namespace(|| "Assign larger value"), self.large_value, LOOKUP_RANGE)?;
            Ok(())
        }
    }

    #[test]
    fn test_range_check_1() {
        /*
            An advantage of unit tests like these is that if you change your circuit configuration
            then the indices of your gates and columns will chnage and your unit tests will fail.
            It gives you some assurance that you're not unkowingly changing a circuit configuration.
         */
        let k = 9;
        const RANGE: usize = 8; // 3-bit value
        const LOOKUP_RANGE: usize = 256; // 8-bit value

        // Successful cases
        for i in 0..RANGE {
            let circuit = MyCircuit::<Fp, RANGE, LOOKUP_RANGE> {
                value: Value::known(Fp::from(i as u64).into()),
                large_value: Value::known(Fp::from(i as u64).into()),
            };

            let prover = MockProver::run(k, &circuit, vec![]).unwrap();
            prover.assert_satisfied();
        }

        // Out-of-range `value = 8`
       /* {
            let circuit = MyCircuit::<Fp, RANGE, LOOKUP_RANGE> {
                value: Value::known(Fp::from(RANGE as u64).into()),
            };
            let prover = MockProver::run(k, &circuit, vec![]).unwrap();
            //prover.assert_satisfied(); it prints out the failure.

            // We can write some unit tests. We hard code some expected failures.
            // In this case, I expect a failure and I know precisely what failure I'm expecting.
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
        }*/
    }

    #[cfg(feature = "dev-graph")]
    #[test]
    fn print_range_check_1() {
        use plotters::prelude::*;

        let root = BitMapBackend::new("range-check-1-layout.png", (1024, 3096)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let root = root
            .titled("Range Check 1 Layout", ("sans-serif", 60))
            .unwrap();

        let circuit = MyCircuit::<Fp, 8> {
            value: Value::unknown(),
        };
        halo2_proofs::dev::CircuitLayout::default()
            .render(3, &circuit, &root)
            .unwrap();
    }
}
