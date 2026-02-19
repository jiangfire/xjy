use xjy::utils::pow::{
    sign_challenge, validate_pow_solution, verify_and_decode_challenge, PowChallenge,
};

fn find_nonce(ch: &PowChallenge) -> String {
    // 低难度用于测试，暴力枚举很快
    for i in 0u64..2_000_000 {
        let nonce = format!("{i}");
        if validate_pow_solution(ch, &nonce).is_ok() {
            return nonce;
        }
    }
    panic!("nonce not found");
}

#[test]
fn pow_roundtrip_and_solution_ok() {
    let secret = b"test_secret".to_vec();
    let now = xjy::utils::pow::now_epoch_seconds();
    let ch = PowChallenge {
        v: 1,
        action: "vote".to_string(),
        target_type: "post".to_string(),
        target_id: 123,
        user_id: 7,
        issued_at: now,
        expires_at: now + 120,
        difficulty: 10,
        salt: "abc".to_string(),
    };

    let token = sign_challenge(&secret, &ch).unwrap();
    let decoded = verify_and_decode_challenge(&secret, &token).unwrap();
    assert_eq!(decoded.target_id, 123);

    let nonce = find_nonce(&decoded);
    validate_pow_solution(&decoded, &nonce).unwrap();
}
