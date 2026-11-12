//! Test harness only. No UI, accounts, network or actual user data.
#[path="../src/startup_gate.rs"] mod startup_gate;
#[path="../src/startup_data.rs"] mod startup_data;
fn main() {println!("Run cargo test --example startup_logic_proof for fixture-only startup checks.");}
