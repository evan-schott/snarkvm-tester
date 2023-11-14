mod test_helpers;

use std::fs::File;
use std::{fs, io, thread};
use std::io::Read;
use std::path::Path;
use std::str::FromStr;
use std::time::Instant;
use test_helpers::*;

use snarkvm;
use snarkvm::prelude::{FromBytes, PrivateKey, TestRng, ToBytes, Value};
use snarkvm::ledger::Ledger;
use snarkvm::prelude::block::{Block, Transaction};
use snarkvm::prelude::Program;

use std::io::Write;
use regex::Regex;


const VERIFY_TX_NUM: usize = 49;
const CREATE_TX_NUM: usize = 36;

fn main() {
    parallel_spam().expect("Failed to spam finalize ops");
    verify_finalize_ops().expect("Failed to spam finalize ops");
}

fn parallel_spam() -> io::Result<()> {
    // Compute parallelization logic
    let mut handles = Vec::new();
    let num_cpus = num_cpus::get();
    let work_per_thread = (CREATE_TX_NUM + num_cpus - 1) / num_cpus; // Round up

    // Check if `./transactions` directory exists
    let transactions_dir = "./transactions";
    fs::create_dir_all(transactions_dir)?;
    let max_number = find_max_transaction_number(transactions_dir)?;

    // Start the timer
    let start = Instant::now();

    // Spawn threads to split workload
    for i in 0..num_cpus {
        let handle = thread::Builder::new()
            .spawn(move || {
               parallel_tx_creator(work_per_thread, i)
            })
            .unwrap(); // Handle potential errors from thread spawning
        handles.push(handle);
    }

    let mut results = Vec::new();

    // Collect the results from each thread
    for handle in handles {
        let result = handle.join().unwrap().unwrap();

        results.push(result);
    }

    let assembled_transaction_list = results.concat();

    for i in 0..assembled_transaction_list.len() {
        let file_path = format!("{}/transaction_{}", transactions_dir, (i as u32)+ 1 + max_number);
        let mut file = File::create(&file_path)?;
        let tx_bytes:Vec<u8> = assembled_transaction_list[i].to_bytes_le().expect("Failed to serialize transaction");
        file.write(&tx_bytes).expect("Failed to write transaction to file");
    }

    // Stop the timer
    let duration = start.elapsed();

    // Print the duration
    println!("Time elapsed is: {:?}", duration);
    println!("Time elapsed per transactions is {:?}", duration / (num_cpus * work_per_thread) as u32);
    println!("Num cpus: {}", num_cpus);
    println!("Work per thread: {}", work_per_thread);

    Ok(())
}
fn parallel_tx_creator(num_jobs: usize, thread_id: usize) -> io::Result<(Vec<Transaction<CurrentNetwork>>)> {
    // Make sure directory ok
    let transactions_dir = "./transactions";
    let rng = &mut TestRng::fixed(6404264900108107703);

    // Initialize the test environment.
    let crate::test_helpers::TestEnv { ledger, private_key, view_key, .. } = crate::test_helpers::sample_test_env(rng);

    // Read child block from bytes
    let file_path = format!("{}/block_child", transactions_dir);
    let mut child_file = File::open(&file_path)?;
    let child_deploy_transfer_block:Block<CurrentNetwork> = Block::<CurrentNetwork>::read_le(&mut child_file).expect("Failed to read child block from file");

    // Check that the next block is valid.
    ledger.check_next_block(&child_deploy_transfer_block).unwrap();

    // Add the deployment block to the ledger.
    ledger.advance_to_next_block(&child_deploy_transfer_block).unwrap();

    // Read parent block from bytes
    let parent_path = format!("{}/block_parent", transactions_dir);
    let mut parent_file = File::open(&parent_path)?;
    let parent_deploy_transfer_block:Block<CurrentNetwork> = Block::<CurrentNetwork>::read_le(&mut parent_file).expect("Failed to read parent block from file");

    // Check that the next block is valid.
    ledger.check_next_block(&parent_deploy_transfer_block).unwrap();

    // Add the deployment block to the ledger.
    ledger.advance_to_next_block(&parent_deploy_transfer_block).unwrap();

    // Read grandfather block from bytes
    let grandfather_path = format!("{}/block_grandfather", transactions_dir);
    let mut grandfather_file = File::open(&grandfather_path)?;
    let grandfather_deploy_transfer_block:Block<CurrentNetwork> = Block::<CurrentNetwork>::read_le(&mut grandfather_file).expect("Failed to read grandfather block from file");

    // Check that the next block is valid.
    ledger.check_next_block(&grandfather_deploy_transfer_block).unwrap();

    // Add the deployment block to the ledger.
    ledger.advance_to_next_block(&grandfather_deploy_transfer_block).unwrap();

    // Complete threads portion of workload
    let mut grandfather_execute_transactions: Vec<Transaction<CurrentNetwork>> = Vec::new();
    for i in 0..num_jobs {
        let r = &mut TestRng::default();
        let execute_inputs: Vec<Value<CurrentNetwork>> = Vec::new();
        let new_tx = ledger.vm().execute(&private_key, ("grandfather_spammer.aleo", "outer_most_call"), execute_inputs.into_iter(), None, 0, None, r)
            .unwrap();
        grandfather_execute_transactions.push(new_tx);

        // Print out progress
        println!("------------------------------------------------------------------");
        println!("Thread: {} has completed {}/{} tasks!", thread_id, i + 1, num_jobs);
        println!("------------------------------------------------------------------");
    }

    Ok(grandfather_execute_transactions)
}

fn verify_finalize_ops() -> io::Result<()> {
    // Make sure directory ok
    let transactions_dir = "./transactions";
    fs::create_dir_all(transactions_dir)?;

    let rng = &mut TestRng::fixed(6404264900108107703);

    // Initialize the test environment.
    let crate::test_helpers::TestEnv { ledger, private_key, view_key, .. } = crate::test_helpers::sample_test_env(rng);

    // Read child block from bytes
    let file_path = format!("{}/block_child", transactions_dir);
    let mut child_file = File::open(&file_path)?;
    let child_deploy_transfer_block:Block<CurrentNetwork> = Block::<CurrentNetwork>::read_le(&mut child_file).expect("Failed to read child block from file");

    // Check that the next block is valid.
    ledger.check_next_block(&child_deploy_transfer_block).unwrap();

    // Add the deployment block to the ledger.
    ledger.advance_to_next_block(&child_deploy_transfer_block).unwrap();

    // Read parent block from bytes
    let parent_path = format!("{}/block_parent", transactions_dir);
    let mut parent_file = File::open(&parent_path)?;
    let parent_deploy_transfer_block:Block<CurrentNetwork> = Block::<CurrentNetwork>::read_le(&mut parent_file).expect("Failed to read parent block from file");

    // Check that the next block is valid.
    ledger.check_next_block(&parent_deploy_transfer_block).unwrap();

    // Add the deployment block to the ledger.
    ledger.advance_to_next_block(&parent_deploy_transfer_block).unwrap();

    // Read grandfather block from bytes
    let grandfather_path = format!("{}/block_grandfather", transactions_dir);
    let mut grandfather_file = File::open(&grandfather_path)?;
    let grandfather_deploy_transfer_block:Block<CurrentNetwork> = Block::<CurrentNetwork>::read_le(&mut grandfather_file).expect("Failed to read grandfather block from file");

    // Check that the next block is valid.
    ledger.check_next_block(&grandfather_deploy_transfer_block).unwrap();

    // Add the deployment block to the ledger.
    ledger.advance_to_next_block(&grandfather_deploy_transfer_block).unwrap();

    // Helper function to assemble grandfather execute transaction
    fn create_transaction(l: &CurrentLedger, pk: &PrivateKey<CurrentNetwork>) -> Transaction<CurrentNetwork> {
        let r = &mut TestRng::default();
        // Append an `grandfather_spam.aleo/outer_most_call` execute transaction to the list of transactions.
        let execute_inputs: Vec<Value<CurrentNetwork>> = Vec::new();
        l.vm().execute(pk, ("grandfather_spammer.aleo", "outer_most_call"), execute_inputs.into_iter(), None, 0, None, r)
            .unwrap()
    }

    let start = Instant::now(); // TODO: Need to modify to only include new ones built

    // Check if `./transactions` directory exists
    let transactions_dir = "./transactions";
    fs::create_dir_all(transactions_dir)?;

    // Spawn threads to split workload
    let mut grandfather_execute_transactions = Vec::new();
    for i in 0..VERIFY_TX_NUM {
        let file_path = format!("{}/transaction_{}", transactions_dir, i);

        if Path::new(&file_path).exists() {
            let mut file = File::open(&file_path)?;
            let mut contents = String::new();
            // TODO: File is bytes. Store them in variable.
            let tx = Transaction::<CurrentNetwork>::read_le(&mut file).expect("Failed to read transaction from file");
            grandfather_execute_transactions.push(tx);
        } else {
            // Create transaction
            let new_tx = create_transaction(&ledger, &private_key);

            // Append to list of transactions
            grandfather_execute_transactions.push(new_tx.clone());

            // Write serialized version to file
            let mut file = File::create(&file_path)?;
            let tx_bytes:Vec<u8> = new_tx.to_bytes_le().expect("Failed to serialize transaction");
            file.write(&tx_bytes).expect("Failed to write transaction to file");
        }



        // Print out progress
        println!("------------------------------------------------------------------");
        println!("------------------------------------------------------------------");
        println!("------------------------------------------------------------------");
        println!("------------------------------------------------------------------");
        println!("{}/{} completed!", i, VERIFY_TX_NUM);
        println!("------------------------------------------------------------------");
        println!("------------------------------------------------------------------");
        println!("------------------------------------------------------------------");
        println!("------------------------------------------------------------------");
    }

    // Construct the next block.
    let grandfather_execute_transfer_block = ledger
        .prepare_advance_to_next_beacon_block(&private_key, vec![], vec![], grandfather_execute_transactions, rng)
        .unwrap();

    // Check that the next block is valid.
    ledger.check_next_block(&grandfather_execute_transfer_block).unwrap();

    // Add the deployment block to the ledger.
    ledger.advance_to_next_block(&grandfather_execute_transfer_block).unwrap();

    // Stop the timer
    let duration = start.elapsed();

    // Print the duration
    println!("Time elapsed is: {:?}", duration);
    println!("Time elapsed per transactions is {:?}", duration / VERIFY_TX_NUM as u32);

    Ok(())
}

fn spam_finalize_ops_parallel() -> io::Result<()> {
    // Make sure directory ok
    let transactions_dir = "./transactions";
    fs::create_dir_all(transactions_dir)?;

    let rng = &mut TestRng::fixed(6404264900108107703);

    // Initialize the test environment.
    let crate::test_helpers::TestEnv { ledger, private_key, view_key, .. } = crate::test_helpers::sample_test_env(rng);

    // Read child block from bytes
    let file_path = format!("{}/block_child", transactions_dir);
    let mut child_file = File::open(&file_path)?;
    let child_deploy_transfer_block:Block<CurrentNetwork> = Block::<CurrentNetwork>::read_le(&mut child_file).expect("Failed to read child block from file");

    // Check that the next block is valid.
    ledger.check_next_block(&child_deploy_transfer_block).unwrap();

    // Add the deployment block to the ledger.
    ledger.advance_to_next_block(&child_deploy_transfer_block).unwrap();

    // Read parent block from bytes
    let parent_path = format!("{}/block_parent", transactions_dir);
    let mut parent_file = File::open(&parent_path)?;
    let parent_deploy_transfer_block:Block<CurrentNetwork> = Block::<CurrentNetwork>::read_le(&mut parent_file).expect("Failed to read parent block from file");

    // Check that the next block is valid.
    ledger.check_next_block(&parent_deploy_transfer_block).unwrap();

    // Add the deployment block to the ledger.
    ledger.advance_to_next_block(&parent_deploy_transfer_block).unwrap();

    // Read grandfather block from bytes
    let grandfather_path = format!("{}/block_grandfather", transactions_dir);
    let mut grandfather_file = File::open(&grandfather_path)?;
    let grandfather_deploy_transfer_block:Block<CurrentNetwork> = Block::<CurrentNetwork>::read_le(&mut grandfather_file).expect("Failed to read grandfather block from file");

    // Check that the next block is valid.
    ledger.check_next_block(&grandfather_deploy_transfer_block).unwrap();

    // Add the deployment block to the ledger.
    ledger.advance_to_next_block(&grandfather_deploy_transfer_block).unwrap();

    // Helper function to assemble grandfather execute transaction
    fn create_transaction(
        l: &CurrentLedger,
        pk: &PrivateKey<CurrentNetwork>,
        r: &mut TestRng,
    ) -> Transaction<CurrentNetwork> {
        // Append an `grandfather_spam.aleo/outer_most_call` execute transaction to the list of transactions.
        let execute_inputs: Vec<Value<CurrentNetwork>> = Vec::new();
        l.vm().execute(pk, ("grandfather_spammer.aleo", "outer_most_call"), execute_inputs.into_iter(), None, 0, None, r)
            .unwrap()
    }

    // Helper function to complete threads portion of workload
    fn complete_threads_portion_of_workload(
        l: &CurrentLedger,
        pk: &PrivateKey<CurrentNetwork>,
        num_jobs: usize,
        thread_id: usize,
    ) -> Vec<Transaction<CurrentNetwork>> {
        let r = &mut TestRng::default();
        let mut grandfather_execute_transactions: Vec<Transaction<CurrentNetwork>> = Vec::new();
        for i in 0..num_jobs {
            grandfather_execute_transactions.push(create_transaction(l, pk, r));

            // Print out progress
            println!("------------------------------------------------------------------");
            println!("Thread: {} has completed {}/{} tasks!", thread_id, i + 1, num_jobs);
            println!("------------------------------------------------------------------");
        }
        grandfather_execute_transactions
    }

    // Compute parallelization logic
    let mut handles = Vec::new();
    let num_cpus = num_cpus::get();
    let work_per_thread = (VERIFY_TX_NUM + num_cpus - 1) / num_cpus; // Round up

    // Check if `./transactions` directory exists
    let transactions_dir = "./transactions";
    fs::create_dir_all(transactions_dir)?;
    let max_number = find_max_transaction_number(transactions_dir)?;

    // Start the timer
    let start = Instant::now();

    let ledger_clone = ledger.clone();
    // Spawn threads to split workload
    for i in 0..num_cpus {
        let ledger_ref = ledger_clone.clone();
        let thread_name = format!("worker-{}", i);
        let handle = thread::Builder::new()
            .name(thread_name) // Setting the thread name
            .spawn(move || {
                complete_threads_portion_of_workload(&ledger_ref, &private_key, work_per_thread, i)
            })
            .unwrap(); // Handle potential errors from thread spawning
        handles.push(handle);
    }

    let mut results = Vec::new();

    // Collect the results from each thread
    for handle in handles {
        let result = handle.join().unwrap();

        results.push(result);
    }

    let assembled_transaction_list = results.concat();

    for i in 0..assembled_transaction_list.len() {
        let file_path = format!("{}/transaction_{}", transactions_dir, (i as u32)+ 1 + max_number);
        let mut file = File::create(&file_path)?;
        let tx_bytes:Vec<u8> = assembled_transaction_list[i].to_bytes_le().expect("Failed to serialize transaction");
        file.write(&tx_bytes).expect("Failed to write transaction to file");
    }

    // Construct the next block.
    let grandfather_execute_transfer_block = ledger
        .prepare_advance_to_next_beacon_block(&private_key, vec![], vec![], assembled_transaction_list, rng)
        .unwrap();

    // Check that the next block is valid.
    ledger.check_next_block(&grandfather_execute_transfer_block).unwrap();

    // Add the deployment block to the ledger.
    ledger.advance_to_next_block(&grandfather_execute_transfer_block).unwrap();

    // Stop the timer
    let duration = start.elapsed();

    // Print the duration
    println!("Time elapsed is: {:?}", duration);
    println!("Time elapsed per transactions is {:?}", duration / (num_cpus * work_per_thread) as u32);
    println!("Num cpus: {}", num_cpus);
    println!("Work per thread: {}", work_per_thread);
    Ok(())
}

fn create_blocks() -> io::Result<()> {
    // Make sure directory ok
    let transactions_dir = "./transactions";
    fs::create_dir_all(transactions_dir)?;

    let rng = &mut TestRng::fixed(6404264900108107703);

    // Initialize the test environment.
    let crate::test_helpers::TestEnv { ledger, private_key, view_key, .. } = crate::test_helpers::sample_test_env(rng);

    // `child_spammer.aleo` source code
    let child_program = Program::<CurrentNetwork>::from_str(
        r"
program child_spammer.aleo;

mapping map:
	key as u8.public;
	value as u8.public;

function spam:
    async spam into r0;
    output r0 as child_spammer.aleo/spam.future;

finalize spam:
    set 0u8 into map[0u8];
    set 1u8 into map[1u8];
    set 2u8 into map[2u8];
    set 3u8 into map[3u8];
    set 4u8 into map[4u8];
    set 5u8 into map[5u8];
    set 6u8 into map[6u8];
    set 7u8 into map[7u8];
    set 8u8 into map[8u8];
    set 9u8 into map[9u8];
    set 10u8 into map[10u8];
    set 11u8 into map[11u8];
    set 12u8 into map[12u8];
    set 13u8 into map[13u8];
    set 14u8 into map[14u8];
    set 15u8 into map[15u8];",
    )
        .unwrap();

    // Create transaction deploying `child_spammer.aleo`
    let child_deploy_transaction = ledger.vm().deploy(&private_key, &child_program, None, 0, None, rng).unwrap();

    // Construct the next block.
    let child_deploy_transfer_block = ledger
        .prepare_advance_to_next_beacon_block(&private_key, vec![], vec![], vec![child_deploy_transaction], rng)
        .unwrap();

    // Cache child block
    let file_path = format!("{}/block_child", transactions_dir);
    let child_block_bytes = child_deploy_transfer_block.to_bytes_le().expect("Can't serialize into child block to bytes");
    let mut child_file = File::create(&file_path)?;
    child_file.write(&child_block_bytes).expect("Failed to write child block to file");

    // Check that the next block is valid.
    ledger.check_next_block(&child_deploy_transfer_block).unwrap();

    // Add the deployment block to the ledger.
    ledger.advance_to_next_block(&child_deploy_transfer_block).unwrap();

    // `parent_spammer.aleo` source code
    let parent_program = Program::<CurrentNetwork>::from_str(
        r"
import child_spammer.aleo;
program parent_spammer.aleo;

function main:
    call child_spammer.aleo/spam into r0;
    call child_spammer.aleo/spam into r1;
    call child_spammer.aleo/spam into r2;
    call child_spammer.aleo/spam into r3;
    call child_spammer.aleo/spam into r4;
    call child_spammer.aleo/spam into r5;
    call child_spammer.aleo/spam into r6;
    call child_spammer.aleo/spam into r7;
    call child_spammer.aleo/spam into r8;
    call child_spammer.aleo/spam into r9;
    call child_spammer.aleo/spam into r10;
    call child_spammer.aleo/spam into r11;
    call child_spammer.aleo/spam into r12;
    call child_spammer.aleo/spam into r13;
    async main r0 r1 r2 r3 r4 r5 r6 r7 r8 r9 r10 r11 r12 r13 into r14;
    output r14 as parent_spammer.aleo/main.future;

finalize main:
    input r0 as child_spammer.aleo/spam.future;
    input r1 as child_spammer.aleo/spam.future;
    input r2 as child_spammer.aleo/spam.future;
    input r3 as child_spammer.aleo/spam.future;
    input r4 as child_spammer.aleo/spam.future;
    input r5 as child_spammer.aleo/spam.future;
    input r6 as child_spammer.aleo/spam.future;
    input r7 as child_spammer.aleo/spam.future;
    input r8 as child_spammer.aleo/spam.future;
    input r9 as child_spammer.aleo/spam.future;
    input r10 as child_spammer.aleo/spam.future;
    input r11 as child_spammer.aleo/spam.future;
    input r12 as child_spammer.aleo/spam.future;
    input r13 as child_spammer.aleo/spam.future;
    await r0;
    await r1;
    await r2;
    await r3;
    await r4;
    await r5;
    await r6;
    await r7;
    await r8;
    await r9;
    await r10;
    await r11;
    await r12;
    await r13;
",
    )
        .unwrap();

    // Create transaction deploying `parent_spammer.aleo`
    let parent_deploy_transaction = ledger.vm().deploy(&private_key, &parent_program, None, 0, None, rng).unwrap();

    // Construct the next block.
    let parent_deploy_transfer_block = ledger
        .prepare_advance_to_next_beacon_block(&private_key, vec![], vec![], vec![parent_deploy_transaction], rng)
        .unwrap();

    // Cache parent block
    let parent_file_path = format!("{}/block_parent", transactions_dir);
    let parent_block_bytes = parent_deploy_transfer_block.to_bytes_le().expect("Can't serialize into parent block to bytes");
    let mut parent_file = File::create(&parent_file_path)?;
    parent_file.write(&parent_block_bytes).expect("Failed to write parent block to file");

    // Check that the next block is valid.
    ledger.check_next_block(&parent_deploy_transfer_block).unwrap();

    // Add the deployment block to the ledger.
    ledger.advance_to_next_block(&parent_deploy_transfer_block).unwrap();
    // `grandfather_spammer.aleo` source code
    let grandfather_program = Program::<CurrentNetwork>::from_str(
        r"
import child_spammer.aleo;
import parent_spammer.aleo;
program grandfather_spammer.aleo;



function outer_most_call:
    call parent_spammer.aleo/main into r0;
    call parent_spammer.aleo/main into r1;
    async outer_most_call r0 r1 into r2;
    output r2 as grandfather_spammer.aleo/outer_most_call.future;

finalize outer_most_call:
    input r0 as parent_spammer.aleo/main.future;
    input r1 as parent_spammer.aleo/main.future;
    await r0;
    await r1;
",
    )
        .unwrap();

    // Create transaction deploying `grandfather_spammer.aleo`
    let grandfather_deploy_transaction =
        ledger.vm().deploy(&private_key, &grandfather_program, None, 0, None, rng).unwrap();

    // Construct the next block.
    let grandfather_deploy_transfer_block = ledger
        .prepare_advance_to_next_beacon_block(&private_key, vec![], vec![], vec![grandfather_deploy_transaction], rng)
        .unwrap();

    // Cache grandfather block
    let grandfather_file_path = format!("{}/block_grandfather", transactions_dir);
    let grandfather_block_bytes = grandfather_deploy_transfer_block.to_bytes_le().expect("Can't serialize into grandfather block to bytes");
    let mut grandfather_file = File::create(&grandfather_file_path)?;
    grandfather_file.write(&grandfather_block_bytes).expect("Failed to write grandfather block to file");

    // Check that the next block is valid.
    ledger.check_next_block(&grandfather_deploy_transfer_block).unwrap();

    // Add the deployment block to the ledger.
    ledger.advance_to_next_block(&grandfather_deploy_transfer_block).unwrap();

    Ok(())
}

fn dummy_file_system_creation() -> io::Result<()> {
    let transactions_dir = "./transactions";
    fs::create_dir_all(transactions_dir)?;

    // Determine the highest number file
    let max_number = find_max_transaction_number(transactions_dir)?;

    for i in 0..VERIFY_TX_NUM {
        let file_path = format!("{}/transaction_{}", transactions_dir, max_number + 1 + (i as u32));

        if Path::new(&file_path).exists() {
            println!("File already exists!");
        } else {
            let mut file = File::create(&file_path)?;
            writeln!(file, "hello {}", max_number + (i as u32))?;
        }
    }
    Ok(())
}

fn find_max_transaction_number(transactions_dir: &str) -> io::Result<u32> {
    let re = Regex::new(r"transaction_(\d+)").unwrap();
    let mut max_num = 0;

    for entry in fs::read_dir(transactions_dir)? {
        if let Ok(entry) = entry {
            if let Some(caps) = re.captures(entry.file_name().to_str().unwrap()) {
                if let Ok(num) = caps[1].parse::<u32>() {
                    if num > max_num {
                        max_num = num;
                    }
                }
            }
        }
    }

    Ok(max_num)
}

fn open_blocks_test() -> io::Result<()> {
    // Make sure directory ok
    let transactions_dir = "./transactions";
    fs::create_dir_all(transactions_dir)?;

    let rng = &mut TestRng::fixed(6404264900108107703);

    // Initialize the test environment.
    let crate::test_helpers::TestEnv { ledger, private_key, view_key, .. } = crate::test_helpers::sample_test_env(rng);

    // Read child block from bytes
    let file_path = format!("{}/block_child", transactions_dir);
    let mut child_file = File::open(&file_path)?;
    let child_deploy_transfer_block:Block<CurrentNetwork> = Block::<CurrentNetwork>::read_le(&mut child_file).expect("Failed to read child block from file");

    // Check that the next block is valid.
    ledger.check_next_block(&child_deploy_transfer_block).unwrap();

    // Add the deployment block to the ledger.
    ledger.advance_to_next_block(&child_deploy_transfer_block).unwrap();

    // Read parent block from bytes
    let parent_path = format!("{}/block_parent", transactions_dir);
    let mut parent_file = File::open(&parent_path)?;
    let parent_deploy_transfer_block:Block<CurrentNetwork> = Block::<CurrentNetwork>::read_le(&mut parent_file).expect("Failed to read parent block from file");

    // Check that the next block is valid.
    ledger.check_next_block(&parent_deploy_transfer_block).unwrap();

    // Add the deployment block to the ledger.
    ledger.advance_to_next_block(&parent_deploy_transfer_block).unwrap();

    // Read grandfather block from bytes
    let grandfather_path = format!("{}/block_grandfather", transactions_dir);
    let mut grandfather_file = File::open(&grandfather_path)?;
    let grandfather_deploy_transfer_block:Block<CurrentNetwork> = Block::<CurrentNetwork>::read_le(&mut grandfather_file).expect("Failed to read grandfather block from file");

    // Check that the next block is valid.
    ledger.check_next_block(&grandfather_deploy_transfer_block).unwrap();

    // Add the deployment block to the ledger.
    ledger.advance_to_next_block(&grandfather_deploy_transfer_block).unwrap();

    Ok(())
}
