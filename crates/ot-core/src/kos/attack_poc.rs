use itybity::ToBits;
use mpz_core::Block;

use crate::{
    kos::{CSP, Receiver, ReceiverConfig, Sender, SenderConfig},
    rcot::{RCOTReceiver, RCOTReceiverOutput, RCOTSender, RCOTSenderOutput},
};

use rand::Rng;
use rand_chacha::ChaCha12Rng;
use rand_core::SeedableRng;

const COUNT: usize = 128;

struct InstanceOutput {
    sender_keys: Vec<Block>,
    receiver_choices: Vec<bool>,
}

fn sender_seeds_for(delta: Block, receiver_seeds: &[[Block; 2]; CSP]) -> [Block; CSP] {
    delta
        .iter_lsb0()
        .zip(receiver_seeds.iter())
        .map(|(b, pair)| if b { pair[1] } else { pair[0] })
        .collect::<Vec<_>>()
        .try_into()
        .unwrap()
}

fn run_instance(
    delta: Block,
    sender_seeds: [Block; CSP],
    receiver_seeds: [[Block; 2]; CSP],
) -> InstanceOutput {
    let mut sender = Sender::new(SenderConfig::default(), delta).setup(sender_seeds);
    let mut receiver = Receiver::new(ReceiverConfig::default()).setup(receiver_seeds);

    sender.alloc(COUNT).unwrap();
    receiver.alloc(COUNT).unwrap();

    while receiver.wants_extend() {
        sender.extend(receiver.extend().unwrap()).unwrap();
    }

    let chi_seed = sender.check_start();
    let receiver_check = receiver.check(chi_seed).unwrap();
    sender.check(receiver_check).unwrap();

    let RCOTSenderOutput { keys, .. } = sender.try_send_rcot(COUNT).unwrap();
    let RCOTReceiverOutput { choices, .. } = receiver.try_recv_rcot(COUNT).unwrap();

    InstanceOutput {
        sender_keys: keys,
        receiver_choices: choices,
    }
}

fn recover_delta(a: &InstanceOutput, b: &InstanceOutput) -> Block {
    for j in 0..COUNT {
        if a.receiver_choices[j] != b.receiver_choices[j] {
            return a.sender_keys[j] ^ b.sender_keys[j];
        }
    }
    panic!("no column with complementary receiver choices")
}

fn gen_delta() -> Block {
    let mut rng = ChaCha12Rng::seed_from_u64(2);
    rng.random::<[u8; 16]>().into()
}

fn gen_receiver_seeds(seed: u64) -> [[Block; 2]; CSP] {
    let mut rng = ChaCha12Rng::seed_from_u64(seed);
    std::array::from_fn(|_| [rng.random(), rng.random()])
}

// A malicious KOS receiver runs two extensions against the same global `delta`
// and reuses the same base OT for both. KOS has no per-instance domain
// separation, so both instances derive identical extension columns; only the
// receiver's (internal) choice bits differ. The sender's correlated keys are
// raw (`key_j = t_j ^ choice_j * delta`), so at any column where the two runs
// chose differently, `key_a ^ key_b == delta`.
#[test]
fn reused_base_ot_leaks_shared_delta() {
    let delta = gen_delta();
    let receiver_seeds = gen_receiver_seeds(3);
    let sender_seeds = sender_seeds_for(delta, &receiver_seeds);

    let a = run_instance(delta, sender_seeds, receiver_seeds);
    let b = run_instance(delta, sender_seeds, receiver_seeds);

    let recovered = recover_delta(&a, &b);

    assert_eq!(
        recovered, delta,
        "malicious receiver recovered the shared delta by reusing the base OT"
    );
}
