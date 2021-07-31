use std;
use rand::Rng;
use rand::distributions::StandardNormal;
use rand::seq::SliceRandom;


const PERCENTAGE_MAX: f64 = 1.0 + std::f64::EPSILON;


fn rand() -> f64 {
    rand::thread_rng().gen::<f64>()
}

// TODO verify that this is correct
pub fn bool() -> bool {
    rand::thread_rng().gen::<bool>()
}

pub fn shuffle<A>(slice: &mut [A]) {
    slice.shuffle(&mut rand::thread_rng())
}

pub fn gaussian() -> f64 {
    rand::thread_rng().sample(StandardNormal)
}

// TODO verify that this is correct
pub fn percentage() -> f64 {
    rand() * PERCENTAGE_MAX
}

// TODO verify that this is correct
pub fn between_exclusive(min: u32, max: u32) -> u32 {
    let range = (max - min) as f64;
    let x = (rand() * range).floor() as u32;
    x + min
}

// TODO verify that this is correct
pub fn between_inclusive(min: u32, max: u32) -> u32 {
    let range = ((max - min) + 1) as f64;
    let x = (rand() * range).floor() as u32;
    x + min
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::{Debug, Error, Formatter};
    use std::cmp::Ordering;
    use std::collections::BTreeMap;

    #[derive(PartialOrd, PartialEq)]
    struct OrdWrap<A>(A);

    impl<A: Debug> Debug for OrdWrap<A> {
        fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
            self.0.fmt(f)
        }
    }

    impl<A: PartialOrd> Ord for OrdWrap<A> {
        fn cmp(&self, other: &Self) -> Ordering {
            self.partial_cmp(other).unwrap()
        }
    }

    impl<A: PartialEq> Eq for OrdWrap<A> {}

    fn test_distribution<F>(name: &str, min: f64, max: f64, mut f: F)
         where //A: ::std::fmt::Display + PartialOrd + PartialEq,
               F: FnMut() -> f64 {
        let mut counts = BTreeMap::new();

        for _ in 0..1000000 {
            *counts.entry(OrdWrap(f())).or_insert(0) += 1;
        }

        const NUMBER_OF_BUCKETS: f64 = 20.0;

        let step = (max - min) / NUMBER_OF_BUCKETS;

        let mut threshold = min + step;
        let mut sum = 0;

        println!("{}:", name);

        for (key, value) in counts {
            let key = key.0;

            assert!(key >= min && key < max, "{} is out of bounds ({} - {})", key, min, max);

            while key > threshold {
                println!("  {} - {}:\n    {}", threshold - step, threshold, sum);
                threshold += step;
                sum = 0;
            }

            // TODO is this the right spot for this ?
            sum += value;
        }

        while threshold <= max {
            println!("  {} - {}:\n    {}", threshold - step, threshold, 0);
            threshold += step;
        }
    }


    /*#[test]
    fn test_bool() {
        test_distribution("bool", || bool());
    }

    #[test]
    fn test_percentage() {
        test_distribution("percentage", || percentage());
    }*/

    #[test]
    fn test_gaussian() {
        test_distribution("gaussian", -6.0, 6.0, || gaussian());
    }
}
