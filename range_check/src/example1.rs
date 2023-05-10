// This helper checks that the value witnessed in a given cell is within a given range.
//layout: an advice column where you witness a value and a selector where you enable the range check constraint.
//      value | q_range_check
// -----------------------------
//      v     |     1
// When writing configs, it's best practice to pass in advice columns beacause advice columns are very often shared across configs. 
use halo2_proofs::{
    plonk::*,
    circuit::{AssignedCell, Layouter, Value},
    arithmetic::FieldExt, poly::Rotation,
};
use std::marker::PhantomData;

#[derive(Debug, Clone)]
struct RangeCheckConfig<F: FieldExt, const RANGE: usize>{
    value: Column<Advice>,
    q_range_check: Selector,
    _marker: PhantomData<F>
}

impl<F: FieldExt, const RANGE: usize> RangeCheckConfig<F, RANGE>{
    fn configure(meta: &mut ConstraintSystem<F>, value: Column<Advice>) -> Self{
        let q_range_check= meta.selector();

        let config= Self{
            q_range_check,
            value,
            _marker: PhantomData
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
        config
    }

    /*
    How can we make the configure and assign APIs better(well) connected?
    They are pretty disjoint. We have to more or less remember the shape in which we configured
    things and manually amke sure that we assign things in that exact shape. That's a lot of overhed
    for the developer
    */
    fn assign(&self, mut layouter: impl Layouter<F>, value: Value<Assigned<F>>) -> Result<(), Error>{
        layouter.assign_region(||"Assign value", |mut region|{
            let offset= 0;
            // Enable q_range_check
            self.q_range_check.enable(&mut region, offset)?;

            //Assign given value
            region.assign_advice(||"assign value", self.value, offset, ||value)?;
            Ok(())

        })
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
    struct MyCircuit<F: FieldExt, const RANGE: usize> {
        value: Value<Assigned<F>>,
    }

    impl<F: FieldExt, const RANGE: usize> Circuit<F> for MyCircuit<F, RANGE> {
        type Config = RangeCheckConfig<F, RANGE>;
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
            config.assign(layouter.namespace(|| "Assign value"), self.value)?;

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
        let k = 4;
        const RANGE: usize = 8; // 3-bit value

        // Successful cases
        for i in 0..RANGE {
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
        }
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
