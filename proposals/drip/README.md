# Commands

## Store code without governance
gaiad tx wasm store artifacts/cw20_base.wasm --from gaia-drip-test --chain-id testdrip-1 --node http://167.71.37.106:26657 --fees 10000uatom --gas 3000000 -y

## Check code
gaiad query wasm list-code --chain-id testdrip-1 --node http://167.71.37.106:26657

## Submit proposal to funding & contract creation
gaiad tx gov submit-proposal ./draft_proposal.json --from gaia-drip-test --chain-id testdrip-1 --node http://167.71.37.106:26657 --fees 750uatom --gas 300000 -y

## Check contract instance
gaiad query wasm list-contract-by-code 1 --chain-id testdrip-1 --node http://167.71.37.106:26657

---
```
Note: need to update salt value for each contract creation.
Default salt example: 
* "test-predictable-contract" in string
* "746573742d7072656469637461626c652d636f6e7472616374" in hex
* "dGVzdC1wcmVkaWN0YWJsZS1jb250cmFjdA==" in base64
```
---

## Alternative contract creation, for testing
gaiad tx wasm instantiate 1 '{"name":"utitan", "symbol":"TITAN", "decimals":6, "initial_balances":[{"address":"cosmos1z04zu8ajwsdgaagx2wv92wu5ks6zlz3e7y77fk", "amount":"1000000000000"}]}' --label "utitan" --from gaia-drip-test --chain-id testdrip-1 --node http://167.71.37.106:26657 --no-admin --fees 1000uatom --gas 200000 -y

### Alternative predictable contract creation, for testing
gaiad tx wasm instantiate2 1 '{"name":"utitan", "symbol":"TITAN", "decimals":6, "initial_balances":[{"address":"cosmos1z04zu8ajwsdgaagx2wv92wu5ks6zlz3e7y77fk", "amount":"1000000000000"}]}' 746573742d7072656469637461626c652d636f6e7472616374 --label "utitan" --from gaia-drip-test --chain-id testdrip-1 --node http://167.71.37.106:26657 --no-admin --fees 1000uatom --gas 200000 -y