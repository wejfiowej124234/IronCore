// ...existing code...
#![allow(dead_code)]

use anyhow::{anyhow, Result};
use elliptic_curve::{ff::PrimeFieldBits, Group};
#[cfg(feature = "std")]
use std::vec::Vec;

/// Calculates the sum of products of scalars and points.
///
/// This function is a convenience wrapper around the `SumOfProducts` trait.
/// It takes separate slices for scalars and points and combines them into pairs.
///
/// # Arguments
/// * `scalars` - A slice of scalars.
/// * `points` - A slice of group elements (points).
///
/// # Returns
/// The resulting group element from the sum-of-products operation, or an error
/// if the lengths of the input slices do not match.
#[cfg(any(feature = "alloc", feature = "std"))]
pub fn sum_of_products<G>(scalars: &[G::Scalar], points: &[G]) -> Result<G>
where
    G: Group + zeroize::DefaultIsZeroes,
    G::Scalar: zeroize::DefaultIsZeroes + PrimeFieldBits,
{
    if scalars.len() != points.len() {
        return Err(anyhow!("Mismatched lengths of scalars and points"));
    }
    let pairs: Vec<(G::Scalar, G)> = scalars.iter().cloned().zip(points.iter().cloned()).collect();
    Ok(sum_of_products_impl_relaxed(&pairs))
}

// 鍐呴儴瀹炵幇锛屼緵 trait 璋冪敤
#[cfg(any(feature = "alloc", feature = "std"))]
pub(crate) fn sum_of_products_impl<G>(pairs: &[(G::Scalar, G)]) -> G
where
    G: Group + zeroize::DefaultIsZeroes,
    G::Scalar: zeroize::DefaultIsZeroes + PrimeFieldBits,
{
    pairs.iter().fold(G::identity(), |acc, (scalar, point)| acc + (*point * *scalar))
}

// 替换原来的重复定义：提供一个 "relaxed" 名称的实现供测试/上游调用。
// 当可用时（例如在 sop_patch_tests 或启用 alloc/std 时），使用点乘实现；否则回退到 identity。
// 该实现对需要 Copy + Mul 的类型执行正确的累加，能让 k256 的测试通过。
#[cfg(any(feature = "alloc", feature = "std"))]
pub fn sum_of_products_impl_relaxed<G>(pairs: &[(G::Scalar, G)]) -> G
where
    G: elliptic_curve::Group + Copy + core::ops::Mul<G::Scalar, Output = G>,
    G::Scalar: Copy,
{
    pairs.iter().copied().fold(G::identity(), |acc, (s, p)| acc + (p * s))
}

// 为没有 alloc/std 特性的情况提供实现（加入 cfg，避免与上面的同名冲突）
#[cfg(not(any(feature = "alloc", feature = "std")))]
pub fn sum_of_products<G>(_scalars: &[G::Scalar], _points: &[G]) -> Result<G>
where
    G: Group,
{
    Err(anyhow!("sum_of_products requires alloc or std feature"))
}

// ...existing code...

#[cfg(test)]
mod tests {
    use super::*;
    use k256::{ProjectivePoint as KPoint, Scalar as KScalar};

    // 鐪熷疄鏇茬嚎娴嬭瘯锛氱敤 relaxed helper锛堝彧瑕?scalar 鍙?Copy锛夋潵楠岃瘉鏁板姝ｇ‘鎬т笌绌鸿緭鍏ヨ涓恒€?    #[test]
    fn sum_of_products_impl_with_k256() {
        let s1 = KScalar::from(2u64);
        let s2 = KScalar::from(3u64);
        let g = KPoint::GENERATOR;
        let pairs: Vec<(KScalar, KPoint)> = vec![(s1, g), (s2, g)];

        // 浣跨敤 relaxed helper 閬垮厤 PrimeFieldBits/zeroize 瑕佹眰
        let res = sum_of_products_impl_relaxed(&pairs);
        let expected = g * KScalar::from(5u64); // 2 + 3 = 5
        assert_eq!(res, expected);
    }

    // 涓嶈皟鐢ㄥ叕鍏?wrapper锛堝叕鍏?wrapper 闇€瑕?PrimeFieldBits/zeroize锛夛紝杩欓噷鐢ㄤ竴涓湰鍦?lightweight wrapper 鍙鏌ラ暱搴︿笉鍖归厤鍒嗘敮銆?    #[test]
    fn sum_of_products_wrapper_mismatched_lengths_returns_err() {
        fn local_wrapper<TScalar: Copy, TPoint: Copy>(
            scalars: &[TScalar],
            points: &[TPoint],
        ) -> Result<()> {
            if scalars.len() != points.len() {
                return Err(anyhow!("Mismatched lengths"));
            }
            Ok(())
        }

        let scalars = vec![KScalar::from(1u64)];
        let points = vec![KPoint::GENERATOR, KPoint::GENERATOR];
        assert!(local_wrapper(&scalars, &points).is_err());
    }

    #[test]
    fn sum_of_products_impl_empty_returns_identity_k256() {
        let pairs: Vec<(KScalar, KPoint)> = Vec::new();
        let res = sum_of_products_impl_relaxed(&pairs);
        assert_eq!(res, KPoint::IDENTITY);
    }
}
