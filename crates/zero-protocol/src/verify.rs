use sha2::{Digest, Sha256};

use crate::{ProtocolError, Tensor};

pub struct OracleVerifier;

impl OracleVerifier {
    /// Compute content hash of a tensor for verification
    pub fn hash_tensor(tensor: &Tensor) -> String {
        let mut hasher = Sha256::new();
        hasher.update(
            serde_json::to_string(&tensor.data)
                .unwrap_or_default()
                .as_bytes(),
        );
        hex::encode(hasher.finalize())
    }

    /// Verify oracle signature (placeholder - checks hash consistency)
    pub fn verify_oracle_response(
        data: &str,
        expected_hash: &str,
    ) -> Result<bool, ProtocolError> {
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let actual_hash = hex::encode(hasher.finalize());
        Ok(actual_hash == expected_hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Tensor, TensorData};

    #[test]
    fn hash_tensor_deterministic_for_same_data() {
        let t = Tensor {
            data: TensorData::Float(vec![1.0, 2.0]),
            confidence: 0.9,
            shape: vec![2],
        };
        let h1 = OracleVerifier::hash_tensor(&t);
        let h2 = OracleVerifier::hash_tensor(&t);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn hash_tensor_differs_when_data_differs() {
        let a = Tensor {
            data: TensorData::String("a".into()),
            confidence: 1.0,
            shape: vec![],
        };
        let b = Tensor {
            data: TensorData::String("b".into()),
            confidence: 1.0,
            shape: vec![],
        };
        assert_ne!(OracleVerifier::hash_tensor(&a), OracleVerifier::hash_tensor(&b));
    }

    #[test]
    fn verify_oracle_response_accepts_matching_hash() {
        let data = "hello";
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let expected = hex::encode(hasher.finalize());
        assert_eq!(
            OracleVerifier::verify_oracle_response(data, &expected).unwrap(),
            true
        );
    }

    #[test]
    fn verify_oracle_response_rejects_mismatch() {
        assert_eq!(
            OracleVerifier::verify_oracle_response(
                "hello",
                "0000000000000000000000000000000000000000000000000000000000000000"
            )
            .unwrap(),
            false
        );
    }
}
