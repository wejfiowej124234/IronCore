#[cfg(feature = "sop_patch_tests")]
mod sop_tests {
    use k256::{AffinePoint as K256Affine, ProjectivePoint as K256Projective};
    use p256::{AffinePoint as P256Affine, ProjectivePoint as P256Projective};

    #[test]
    fn serdes_roundtrip_k256_affine() {
        let point_affine: K256Affine = K256Projective::GENERATOR.to_affine();
        let json = serde_json::to_string(&point_affine).expect("json serialize failed");
        let got: K256Affine = serde_json::from_str(&json).expect("json deserialize failed");
        assert_eq!(point_affine, got);
    }

    #[test]
    fn serdes_roundtrip_k256_affine_vec() {
        let point_affine: K256Affine = K256Projective::GENERATOR.to_affine();
        let vec = vec![point_affine];
        let json = serde_json::to_string(&vec).expect("json serialize failed");
        let got: Vec<K256Affine> = serde_json::from_str(&json).expect("json deserialize failed");
        assert_eq!(vec, got);
    }

    #[test]
    fn serdes_roundtrip_p256_affine() {
        let point_affine: P256Affine = P256Projective::GENERATOR.to_affine();
        let json = serde_json::to_string(&point_affine).expect("json serialize failed");
        let got: P256Affine = serde_json::from_str(&json).expect("json deserialize failed");
        assert_eq!(point_affine, got);
    }

    #[test]
    fn serdes_roundtrip_p256_affine_vec() {
        let point_affine: P256Affine = P256Projective::GENERATOR.to_affine();
        let vec = vec![point_affine];
        let json = serde_json::to_string(&vec).expect("json serialize failed");
        let got: Vec<P256Affine> = serde_json::from_str(&json).expect("json deserialize failed");
        assert_eq!(vec, got);
    }
}
