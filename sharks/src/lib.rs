//! Fast, small and secure [Shamir's Secret
//! Sharing](https://en.wikipedia.org/wiki/Shamir%27s_Secret_Sharing)
//! library crate for large finite fields
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

// implement operations using a larger finite field as well
extern crate ff;
mod share_ff;

use alloc::vec::Vec;
use core::convert::TryInto;
use hashbrown::HashSet;

use crate::ff::PrimeField;
pub use share_ff::Evaluator;
pub use share_ff::Share;
pub use share_ff::{get_evaluator, interpolate, random_polynomial};
pub use share_ff::{Fp, FpRepr, FIELD_ELEMENT_LEN};

/// Tuple struct which implements methods to generate shares and recover secrets over a 256 bits Galois Field.
/// Its only parameter is the minimum shares threshold.
pub struct Sharks(pub u32);

impl Sharks {
    /// This method is useful when `std` is not available. For typical usage
    /// see the `dealer` method.
    ///
    /// Given a `secret` byte slice, returns an `Iterator` along new shares.
    /// The maximum number of shares that can be generated is 256.
    /// A random number generator has to be provided.
    ///
    /// Example:
    /// ```
    /// # use sharks::{ Sharks, Share };
    /// # use rand_chacha::rand_core::SeedableRng;
    /// # let sharks = Sharks(3);
    /// // Obtain an iterator over the shares for secret [1, 2]
    /// let mut rng = rand_chacha::ChaCha8Rng::from_seed([0x90; 32]);
    /// let dealer = sharks.dealer_rng(&[1, 2], &mut rng);
    /// // Get 3 shares
    /// let shares: Vec<Share> = dealer.take(3).collect();
    pub fn dealer_rng<R: rand::Rng>(
        &self,
        secret: &[u8],
        rng: &mut R,
        //) -> impl Iterator<Item = Share> {
    ) -> Evaluator {
        let mut polys = Vec::with_capacity(secret.len());

        let secret_fp_len = secret.len() / FIELD_ELEMENT_LEN;
        for i in 0..secret_fp_len {
            let element = Fp::from_repr(FpRepr(
                secret[i * FIELD_ELEMENT_LEN..(i + 1) * FIELD_ELEMENT_LEN]
                    .try_into()
                    .expect("bad chunk"),
            ))
            .unwrap();
            polys.push(random_polynomial(element, self.0, rng));
        }

        get_evaluator(polys)
    }

    /// Given a `secret` byte slice, returns an `Iterator` along new shares.
    /// The maximum number of shares that can be generated is 256.
    ///
    /// Example:
    /// ```
    /// # use sharks::{ Sharks, Share };
    /// # let sharks = Sharks(3);
    /// // Obtain an iterator over the shares for secret [1, 2]
    /// let dealer = sharks.dealer(&[1, 2]);
    /// // Get 3 shares
    /// let shares: Vec<Share> = dealer.take(3).collect();
    #[cfg(feature = "std")]
    pub fn dealer(&self, secret: &[u8]) -> Evaluator {
        let mut rng = rand::thread_rng();
        self.dealer_rng(secret, &mut rng)
    }

    /// Given an iterable collection of shares, recovers the original secret.
    /// If the number of distinct shares is less than the minimum threshold an `Err` is returned,
    /// otherwise an `Ok` containing the secret.
    ///
    /// Example:
    /// ```
    /// # use sharks::{ Sharks, Share };
    /// # use rand_chacha::rand_core::SeedableRng;
    /// # let sharks = Sharks(3);
    /// # let mut rng = rand_chacha::ChaCha8Rng::from_seed([0x90; 32]);
    /// # let mut shares: Vec<Share> = sharks.dealer_rng(&[1], &mut rng).take(3).collect();
    /// // Recover original secret from shares
    /// let mut secret = sharks.recover(&shares);
    /// // Secret correctly recovered
    /// assert!(secret.is_ok());
    /// // Remove shares for demonstration purposes
    /// shares.clear();
    /// secret = sharks.recover(&shares);
    /// // Not enough shares to recover secret
    /// assert!(secret.is_err());
    pub fn recover<'a, T>(&self, shares: T) -> Result<Vec<u8>, &str>
    where
        T: IntoIterator<Item = &'a Share>,
        T::IntoIter: Iterator<Item = &'a Share>,
    {
        let mut share_length: Option<usize> = None;
        let mut keys: HashSet<Vec<u8>> = HashSet::new();
        let mut values: Vec<Share> = Vec::new();

        for share in shares.into_iter() {
            if share_length.is_none() {
                share_length = Some(share.y.len());
            }

            if Some(share.y.len()) != share_length {
                return Err("All shares must have the same length");
            } else if keys.insert(share.x.to_repr().as_ref().to_vec()) {
                values.push(share.clone());
            }
        }

        if keys.is_empty() || (keys.len() < self.0 as usize) {
            Err("Not enough shares to recover original secret")
        } else {
            // We only need the threshold number of shares to recover
            Ok(interpolate(&values[0..self.0 as usize]))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Fp, Share, Sharks};
    use crate::ff::{Field, PrimeField};
    use alloc::{vec, vec::Vec};

    impl Sharks {
        #[cfg(not(feature = "std"))]
        fn make_shares(&self, secret: &[u8]) -> impl Iterator<Item = Share> {
            use rand_chacha::{rand_core::SeedableRng, ChaCha8Rng};

            let mut rng = ChaCha8Rng::from_seed([0x90; 32]);
            self.dealer_rng(secret, &mut rng)
        }

        #[cfg(feature = "std")]
        fn make_shares(&self, secret: &[u8]) -> impl Iterator<Item = Share> {
            self.dealer(secret)
        }
    }

    fn fp_one() -> Fp {
        Fp::one()
    }

    fn fp_two() -> Fp {
        fp_one().double()
    }

    fn fp_one_repr() -> Vec<u8> {
        (Fp::one()).to_repr().as_ref().to_vec()
    }

    fn fp_two_repr() -> Vec<u8> {
        (fp_one().double()).to_repr().as_ref().to_vec()
    }

    fn fp_three_repr() -> Vec<u8> {
        (fp_two() + fp_one()).to_repr().as_ref().to_vec()
    }

    fn fp_four_repr() -> Vec<u8> {
        (fp_two() + fp_two()).to_repr().as_ref().to_vec()
    }

    #[test]
    fn test_insufficient_shares_err() {
        let sharks = Sharks(500);
        let shares: Vec<Share> = sharks.make_shares(&fp_one_repr()).take(499).collect();
        let secret = sharks.recover(&shares);
        assert!(secret.is_err());
    }

    #[test]
    fn test_duplicate_shares_err() {
        let sharks = Sharks(500);
        let mut shares: Vec<Share> = sharks.make_shares(&fp_one_repr()).take(500).collect();
        shares[1] = Share {
            x: shares[0].x,
            y: shares[0].y.clone(),
        };
        let secret = sharks.recover(&shares);
        assert!(secret.is_err());
    }

    #[test]
    fn test_integration_works() {
        let sharks = Sharks(500);
        let mut input = Vec::new();
        input.extend(fp_one_repr());
        input.extend(fp_two_repr());
        input.extend(fp_three_repr());
        input.extend(fp_four_repr());
        let shares: Vec<Share> = sharks.make_shares(&input).take(500).collect();
        let secret = sharks.recover(&shares).unwrap();
        assert_eq!(secret, get_test_bytes());
    }

    use core::iter;
    #[test]
    fn test_integration_works_random() {
        let sharks = Sharks(40);
        let mut rng = rand::thread_rng();
        let mut input = Vec::new();
        input.extend(fp_one_repr());
        input.extend(fp_two_repr());
        input.extend(fp_three_repr());
        input.extend(fp_four_repr());
        let evaluator = sharks.dealer(&input);
        let shares: Vec<Share> = iter::repeat_with(|| evaluator.gen(&mut rng))
            .take(55)
            .collect();
        //let shares: Vec<Share> = sharks.make_shares(&[1, 2, 3, 4]).take(255).collect();
        let secret = sharks.recover(&shares).unwrap();
        assert_eq!(secret, get_test_bytes());
    }

    fn get_test_bytes() -> Vec<u8> {
        let suffix = vec![0u8; 31];
        let mut bytes = vec![1u8; 1];
        bytes.extend(suffix.clone()); // x coord
        bytes.extend(vec![2u8; 1]);
        bytes.extend(suffix.clone()); // y coord #1
        bytes.extend(vec![3u8; 1]);
        bytes.extend(suffix.clone()); // y coord #2
        bytes.extend(vec![4u8; 1]);
        bytes.extend(suffix); // y coord #3
        bytes
    }
}
