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
    receiver_msgs: Vec<Block>,
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
    instance_id: Block,
    sender_seeds: [Block; CSP],
    receiver_seeds: [[Block; 2]; CSP],
) -> InstanceOutput {
    let mut sender = Sender::new(SenderConfig::default(), delta, instance_id).setup(sender_seeds);
    let mut receiver = Receiver::new(ReceiverConfig::default(), instance_id).setup(receiver_seeds);

    sender.alloc(COUNT).unwrap();
    receiver.alloc(COUNT).unwrap();

    while receiver.wants_extend() {
        sender.extend(receiver.extend().unwrap()).unwrap();
    }

    let chi_seed = sender.check_start();
    let receiver_check = receiver.check(chi_seed).unwrap();
    sender.check(receiver_check).unwrap();

    let RCOTSenderOutput { keys, .. } = sender.try_send_rcot(COUNT).unwrap();
    let RCOTReceiverOutput { choices, msgs, .. } = receiver.try_recv_rcot(COUNT).unwrap();

    InstanceOutput {
        sender_keys: keys,
        receiver_choices: choices,
        receiver_msgs: msgs,
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

#[test]
fn domain_separation_must_prevent_delta_recovery_under_seed_precompensation() {
    let delta = gen_delta();
    let receiver_seeds_a = gen_receiver_seeds(3);
    let sender_seeds_a = sender_seeds_for(delta, &receiver_seeds_a);

    let id_a: Block = [7u8; 16].into();
    let id_b: Block = [11u8; 16].into();
    assert_ne!(id_a, id_b);

    let tweak = id_a ^ id_b;
    let receiver_seeds_b: [[Block; 2]; CSP] =
        std::array::from_fn(|i| [receiver_seeds_a[i][0] ^ tweak, receiver_seeds_a[i][1] ^ tweak]);
    let sender_seeds_b = sender_seeds_for(delta, &receiver_seeds_b);

    let a = run_instance(delta, id_a, sender_seeds_a, receiver_seeds_a);
    let b = run_instance(delta, id_b, sender_seeds_b, receiver_seeds_b);

    let recovered = recover_delta(&a, &b);

    // Δ comes from the sender's keys; the receiver's own msgs are Δ-free.
    let j = (0..COUNT)
        .find(|&j| a.receiver_choices[j] != b.receiver_choices[j])
        .unwrap();
    println!("receiver-only msg XOR = {:?}", a.receiver_msgs[j] ^ b.receiver_msgs[j]);
    println!("true Δ      = {:?}", delta);
    println!("recovered Δ = {:?}", recovered);

    assert_ne!(
        recovered, delta,
        "malicious receiver recovered delta across two domain-separated instances"
    );
}
