### Build
`cargo +nightly build --release`  
`./target/release/node-template --dev`

### Test
1. Navigate to https://polkadot.js.org/apps/#/explorer
2. Click the lef top icon to open settings and choose the local node, just like below
![alt select-node](images/select-node.png)
3. Click the developer tab, choose Extrinsics, and then use Alice to start a new round
![alt star-round](images/start-round.png)
4. You can then run register_project or donate
![alt register-project](images/register-project.png)
5. After that, you can use any account with balance to vote and watch the changes