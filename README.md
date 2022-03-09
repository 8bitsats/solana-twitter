# solana-twitter

## Solana and Anchor commands

### Show solnana key pair

```
solana address
```

### Generate Key

```
solana-keygen new
```

### Show programID

```
solana address -k target/deploy/solana_twitter-keypair.json
```

### Start local ledger

```
solana-test-validator --reset
```

### Run and Deploy

```
anchor build
anchor deploy
```

### Special commands for a cycle

- `anchor localnet`

```
solana-test-validator --reset
anchor build
anchor deploy
```

- `ahnchor test`

```
solana-test-validator --reset
anchor build
anchor deploy
anchor run test
```

## Structuring our Tweet account

### Everything is an account

- In solidity,
  - bunch of code storing bunch of data and interact with it
  - Any user that interacts with a smart contract ends up updating data inside a smart contract
- In Solana,
  - store data somewhere -> Should creat new account
  - account: little clouds of data
  - big account storing all the information, or many little accounts
  - Programs are also speical accounts storing their own code, read-only, executable
  - Programs, Wallets, NFTs, Tweets and everyting

### Create scalable account

- Every tweet stored on its own small account

```rs
#[account]
pub struct Tweet {
    pub author: Pubkey,
    pub timestamp: i64,
    pub topic: String,
    pub content: String,
}
```

- `#[account]`: a custom rust attribute by Anchor to parse account to and from an array of bytes
- `author`: public key to track the publisher
  - owner of Tweet is Solana-Twitter Program not the publisher
  - to track the publisher, need public key
- `timestamp`: the time the tweet was published
- `topic`: topic from hashtags
- `content`: the payload

## Rent

- To adds data to the blockchain, pay fee proportional to the size of the account.
- When the account runs out of money, the account is deleted

### Rent-exempt

- Add enough money in the account to pay the equivalent of two years of rent -> rent-exempt
  - the money will stay on the account forever and wil never be collected.
- when close the account, will get back the rent-exempt money

```sh
solana rent 4000
# Outputs:
# Rent per byte-year: 0.00000348 SOL
# Rent per epoch: 0.000078662 SOL
# Rent-exempt minimum: 0.02873088 SOL
```

### Discriminator

- Whenever a new account is created, a discriminator of exactly 8 bytes will be added

```rs
const DISCRIMINATOR_LENGTH: usize = 8;
```

### Author

- Pubkey type -> 32 bytes

```rs
const PUBLIC_KEY_LENGTH: usize = 32;
```

### Timestamp

- i64 -> 8bytes

```rs
const TIMESTAMP_LENGTH: usize = 8;
```

### Topic

- String -> Vec<u8>
- Let's say max size of 50 chars \* 4bytes of UTF-8
- `vec prefix` 4bytes for total length

```rs
const STRING_LENGTH_PREFIX: usize = 4; // Stores the size of the string.
const MAX_TOPIC_LENGTH: usize = 50 * 4; // 50 chars max.
```

### Content

- Let's say max 280 chars \* 4(UTF-8) + 4(vec prefix)

```rs
const MAX_CONTENT_LENGTH: usize = 280 * 4; // 280 chars max.
```

### Add LEN constant on the Tweet

```rs
impl Tweet {
    const LEN: usize = DISCRIMINATOR_LENGTH
        + PUBLIC_KEY_LENGTH // Author.
        + TIMESTAMP_LENGTH // Timestamp.
        + STRING_LENGTH_PREFIX + MAX_TOPIC_LENGTH // Topic.
        + STRING_LENGTH_PREFIX + MAX_CONTENT_LENGTH; // Content.
}
```

## Our first instruction

### Defining the context

- Programs in Solana are stateless -> requires providing all the necessary context
- Context?:
  - its public key should be provided when sending the instruction
  - use its private key to sign the instruction

```rs
#[derive(Accounts)]
pub struct SendTweet<'info> {
    pub tweet: Account<'info, Tweet>,
    pub author: Signer<'info>,
    pub system_program: AccountInfo<'info>,
}
```

- `tweet`: tweetAccount{author, timestamp, topic, content}
- `author`: who is sending, signature to prove it
- `system_program`: cuz of stateless, even system should be in context

### Account constraints

- help us with security access control and initialize
- should provide space size
- author should pay rent-exempt -> mut

```rs
#[derive(Accounts)]
pub struct SendTweet<'info> {
    #[account(init, payer = author, space = Tweet::LEN)]
    pub tweet: Account<'info, Tweet>,
    #[account(mut)]
    pub author: Signer<'info>,
    #[account(address = system_program::ID)]
    pub system_program: AccountInfo<'info>,
}
```

### Implementing the logic

```rs
pub fn send_tweet(ctx: Context<SendTweet>, topic: String, content: String) -> ProgramResult {
    let tweet: &mut Account<Tweet> = &mut ctx.accounts.tweet;
    let author: &Signer = &ctx.accounts.author;
    let clock: Clock = Clock::get().unwrap();

    tweet.author = *author.key;
    tweet.timestamp = clock.unix_timestamp;
    tweet.topic = topic;
    tweet.content = content;

    Ok(())
}
```

- `topic`, `content`: Any argument which is not an account can be provided after `ctx`

### Guarding against invalid data

```rs
if topic.chars().count() > 50 {
    return Err(error!(ErrorCode::TopicTooLong));
}

if content.chars().count() > 280 {
    return Err(error!(ErrorCode::ContentTooLong));
}
```

### Instruction vs transaction

- `a transaction`(`tx`) is composed of one or multiple `instructions`(`ix`)

## Testing our instruction

### Overview

- JSON RPC API
- Program
  - Provider: `@project-serum/anchor`
    - Connection encapsulated by Cluster `@solana/web3.js`
    - Wallet: accesses to the key pair
  - IDL(Interfaace Description Language): structured description of program including pubKey

### A client just for tests

- provider configurations

```rs
// Anchor.toml
[provider]
cluster = "localnet"
wallet = "~/.config/solana/id.json"
```

### Sending a tweet

```ts
// tests/solana-twitter.ts
it("can send a new tweet", async () => {
  // Call the "SendTweet" instruction.
  const tweet = anchor.web3.Keypair.generate();
  await program.rpc.sendTweet("veganism", "Hummus, am I right?", {
    accounts: {
      tweet: tweet.publicKey,
      author: program.provider.wallet.publicKey,
      systemProgram: anchor.web3.SystemProgram.programId,
    },
    signers: [tweet],
  });

  // Fetch the account details of the created tweet.
  const tweetAccount = await program.account.tweet.fetch(tweet.publicKey);
  console.log(tweetAccount);
});
```

### Sending a tweet without a topic

### Sending a tweet from a different author

- `lamport`: the smallest decimal of Solana's native token
- 1 SOL = 1'000'000'000 lamports
- should airdrop money to otherUser
- every time a new local ledger is created, it automatically airdrops 500 million SOL to your local wallet

### Testing our custom guards

- topic with more than 50 characters
- topic with more than 280 characters

## Fetching tweets from the program

### Fetching all tweets

```rs
const tweetAccounts = await program.account.tweet.all();
assert.equal(tweetAccounts.length, 3);
```

### Filtering tweets by author

- The dataSize filter

```
{
    dataSize: 2000,
}
```

- The memcmp filter

```ts
{
    memcmp: {
        offset: 42, // Starting from the 42nd byte for example.
        bytes: 'B1AfN7AgpMyctfFbjmvRAvE1yziZFDb9XCwydBjJwtRN', // My base-58 encoded public key.
    }
}
```

- Use the memcmp filter on the author's public key
  - right after 8 bytes of discriminator

```ts
const tweetAccounts = await program.account.tweet.all([
  {
    memcmp: {
      offset: 8, // Discriminator.
      bytes: authorPublicKey.toBase58(),
    },
  },
]);
```

- fetch(pubKey) vs all()

```ts
// fetch(pubKey) -> Tweet account with all of its data parsed
program.account.tweet.fetch(tweet.publicKey);
// all() -> each Tweet accounts with pubkey
await program.account.tweet.all().every(tweetAccount => {
    return (
      tweetAccount.account.author.toBase58() === authorPublicKey.toBase58()
    );
}
```

### Filtering tweets by topic

- Discriminator + Author public key + Timestamp + Topic string prefix
- encode with `bs58`

```ts
const tweetAccounts = await program.account.tweet.all([
  {
    memcmp: {
      offset:
        8 + // Discriminator.
        32 + // Author public key.
        8 + // Timestamp.
        4, // Topic string prefix.
      bytes: bs58.encode(Buffer.from("veganism")),
    },
  },
]);
```
