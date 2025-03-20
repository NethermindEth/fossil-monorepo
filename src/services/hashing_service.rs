// read from fossil light client to see if it has all the avg_hourly_block_fee that we want
// if so continue, 
// if not we need to wait for the fossil light client to catch up,
// maybe we need to update the status and allow PL to recall

// check the batch of hashes, 
// for batches that are not available, we need to make a transaction to store it
// once this is done, we make another transaction to hash the batch of hashes
