//! Copy of `nrvo-simple.rs`, to ensure that full dest-prop handles it too.

// EMIT_MIR rustc.nrvo.DestinationPropagation.diff
fn nrvo(init: fn(&mut [u8; 1024])) -> [u8; 1024] {
    let mut buf = [0; 1024];
    init(&mut buf);
    buf
}

fn main() {
    let _ = nrvo(|buf| {
        buf[4] = 4;
    });
}
