use crate::Ledger;
use snarkvm::console::{
    account::{Address, PrivateKey, ViewKey},
    network::Testnet3,
    prelude::*,
};
use snarkvm::ledger::store::ConsensusStore;
use snarkvm::prelude::block::Block;
use snarkvm::synthesizer::vm::VM;
pub(crate) type CurrentNetwork = Testnet3;

pub(crate) type CurrentLedger =
Ledger<CurrentNetwork, snarkvm::ledger::store::helpers::memory::ConsensusMemory<CurrentNetwork>>;
pub(crate) type CurrentConsensusStore =
ConsensusStore<CurrentNetwork, snarkvm::ledger::store::helpers::memory::ConsensusMemory<CurrentNetwork>>;
pub(crate) struct TestEnv {
    pub ledger: CurrentLedger,
    pub private_key: PrivateKey<CurrentNetwork>,
    pub view_key: ViewKey<CurrentNetwork>,
    pub address: Address<CurrentNetwork>,
}

pub(crate) fn sample_test_env(rng: &mut (impl Rng + CryptoRng)) -> TestEnv {
    // Sample the genesis private key.
    let private_key = PrivateKey::<CurrentNetwork>::new(rng).unwrap();
    let view_key = ViewKey::try_from(&private_key).unwrap();
    let address = Address::try_from(&private_key).unwrap();
    // Sample the ledger.
    let ledger = sample_ledger(private_key, rng);
    // Return the test environment.
    TestEnv { ledger, private_key, view_key, address }
}

pub(crate) fn sample_genesis_block() -> Block<CurrentNetwork> {
    Block::<CurrentNetwork>::from_bytes_le(CurrentNetwork::genesis_bytes()).unwrap()
}

pub(crate) fn sample_ledger(
    private_key: PrivateKey<CurrentNetwork>,
    rng: &mut (impl Rng + CryptoRng),
) -> CurrentLedger {
    // Initialize the store.
    let store = CurrentConsensusStore::open(None).unwrap();
    // Create a genesis block.
    let genesis = VM::from(store).unwrap().genesis_beacon(&private_key, rng).unwrap();
    // Initialize the ledger with the genesis block.
    let ledger = CurrentLedger::load(genesis.clone(), None).unwrap();
    // Ensure the genesis block is correct.
    assert_eq!(genesis, ledger.get_block(0).unwrap());
    // Return the ledger.
    ledger
}