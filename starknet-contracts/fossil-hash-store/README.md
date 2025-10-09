Currently, the project is built using Scarb v2.9.2 because it is using Sierra compiler of version v.1.6.0.

We are required to use an older version of Scarb to build the project because the local node `starknet-devnet` currently only supports Sierra v1.6.0.

In Scarb.toml, we have also temporary set the following to be compatible:
```
assert_macros = "2.9.2"
starknet = "2.9.2"

openzeppelin_access = "0.20.0"
openzeppelin_upgrades = "0.20.0"
```

## Prerequisite to compile and test
[Starknet devnet](https://docs.starknet.io/quick-start/environment-setup/#installing-devnet)

[Scarb](https://docs.swmansion.com/scarb/) 

[Starkli](https://docs.starknet.io/quick-start/environment-setup/#installing-starkli) 

[Starknet foundry](https://foundry-rs.github.io/starknet-foundry/getting-started/installation.html) 

## Deploying to Starknet
### Generating account 
[Generating an account for local deployment](https://docs.starknet.io/quick-start/devnet/#fetching_a_predeployed_account) 

[Deploying a new account to Sepolia](https://docs.starknet.io/quick-start/sepolia/#deploying_a_new_sepolia_account) 


### Creating env file
Create a `.env.<network>` file with the following variables:

(refer to `.env.example` for the variables)

```
STARKNET_RPC=
STARKNET_ACCOUNT=
STARKNET_PRIVATE_KEY=
OWNER_ADDRESS=
FOSSIL_VERIFIER_ADDRESS=
```

network can be `sepolia` or `mainnet`


### Run the deploy script
change permission
```
chmod +x scripts/deploy-starknet.sh
```

run command 
```
./scripts/deploy-starknet.sh <network>
```

