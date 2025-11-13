//! 该文件依赖 SumOfProducts 对 Scalar 的 PrimeFieldBits 约束，k256::Scalar 未实现。
//! 默认禁用；如需启用，请在支持的曲线上开启 feature: `--features sop_patch_tests`.

#![cfg(feature = "sop_patch_tests")]

use elliptic_curve_tools::sum_of_products_impl_relaxed;
use k256::{ProjectivePoint, Scalar};

#[test]
fn sop_compiles() {
    let pairs: Vec<(Scalar, ProjectivePoint)> = vec![];
    // 调用 patch crate 中放宽约束的 helper 函数
    let _ = sum_of_products_impl_relaxed(&pairs);
}
