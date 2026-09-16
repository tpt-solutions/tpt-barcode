extern crate alloc;
#[test]
fn scratch_runs_round_trip() {
    let bc = tpt_barcode_1d::code128::encode_b(b"HELLO-128").unwrap();
    let runs: alloc::vec::Vec<u32> = bc.modules.iter().map(|&w| w as u32).collect();
    match tpt_barcode_1d::runs::decode_code128_runs(&runs) {
        Ok(s) => println!("decoded via runs: {s}"),
        Err(e) => println!("runs decode error: {e:?}"),
    }
}
