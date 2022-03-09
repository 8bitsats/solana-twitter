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

- `anchor localnet`: keep local ledger

```
solana-test-validator --reset
anchor build
anchor deploy
```

- `anchor run test`: pre-defined script to test

- `anchor test`: test and end ledger

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

## Scaffolding the frontend

### Install Vue CLI

```
npm install -g @vue/cli@5.0.0-rc.1
vue create app --force
npm install @solana/web3.js @project-serum/anchor
```

- To be safe from confusing polyfill errors

```ts
// vue.config.js
onst webpack = require('webpack')
const { defineConfig } = require('@vue/cli-service')

module.exports = defineConfig({
    transpileDependencies: true,
    configureWebpack: {
        plugins: [
            new webpack.ProvidePlugin({
                Buffer: ['buffer', 'Buffer']
            })
        ],
        resolve: {
            fallback: {
                crypto: false,
                fs: false,
                assert: false,
                process: false,
                util: false,
                path: false,
                stream: false,
            }
        }
    }
})
```

### Configure ESLint

```
"eslintConfig": {
  "root": true,
  "env": {
    "node": true,
    "vue/setup-compiler-macros": true
  },
  "extends": [
    "plugin:vue/vue3-essential",
    "eslint:recommended"
  ],
  "parserOptions": {
    "parser": "@babel/eslint-parser"
  },
  "rules": {
    "vue/script-setup-uses-vars": "error"
  }
},
```

### Install TailwindCSS

```
npm install tailwindcss@latest postcss@latest autoprefixer@latest
npx tailwindcss init -p
```

```ts
// tailwind.config.js
module.exports = {
  purge: ["./public/index.html", "./src/**/*.{vue,js,ts,jsx,tsx}"],
...
```

```css
/* touch src/main.css */
@tailwind base;
@tailwind components;
@tailwind utilities;
```

```js
// main.js
import "./main.css";
...

```

### Install Vue Router

```
npm install vue-router@4
```

```js
touch src/routes.js

export default [
    {
        name: 'Home',
        path: '/',
        component: require('@/components/PageHome').default,
    },
    {
        name: 'Topics',
        path: '/topics/:topic?',
        component: require('@/components/PageTopics').default,
    },
    {
        name: 'Users',
        path: '/users/:author?',
        component: require('@/components/PageUsers').default,
    },
    {
        name: 'Profile',
        path: '/profile',
        component: require('@/components/PageProfile').default,
    },
    {
        name: 'Tweet',
        path: '/tweet/:tweet',
        component: require('@/components/PageTweet').default,
    },
    {
        name: 'NotFound',
        path: '/:pathMatch(.*)*',
        component: require('@/components/PageNotFound').default,
    },
]
```

```js
// main.js
// Routing.
import { createRouter, createWebHashHistory } from "vue-router";
import routes from "./routes";
const router = createRouter({
  history: createWebHashHistory(),
  routes,
});

// Create the app.
import { createApp } from "vue";
import App from "./App.vue";
createApp(App).use(router).mount("#app");
```

### Components

App.vue: This is the main component that loads when our application starts. It designs the overall layout of our app and delegates the rest to Vue Router by using the <router-view> component. Any page that matches the current URL will be rendered where <router-view> is.

- PageHome.vue: The home page. It contains a form to send tweets and lists the latest tweets from everyone.
- PageNotFound.vue: The 404 fallback page. It displays an error message and offers to go back to the home page.
- PageProfile.vue: The profile page for the connected user/wallet. It displays the wallet’s public key before showing the tweet form and the list of tweets sent from that wallet.
- PageTopics.vue: The topics page allows users to enter a topic and displays all tweets matching it. Once a topic is entered it also displays a form to send tweets with that topic pre-filled.
- PageTweet.vue: The tweet page only shows one tweet. The tweet’s public key is provided in the URL allowing us to fetch the tweet account. This is useful for users to share tweets.
- PageUsers.vue: Similarly to the topics page, the users page allows searching for other users by entering their public key. When a valid public key is entered, all tweets from that user will be fetched and displayed on this page.
- TheSidebar.vue: This component is used in the main App.vue component and designs the sidebar on the left of the app. It uses the <router-link> component to easily generate Vue Router URLs. It also contains a button for users to connect their wallets but for now, that button doesn’t do anything.
- TweetCard.vue: This component is responsible for the design of one tweet. It is used everywhere we need to display tweets.
- TweetForm.vue: This component designs the form allowing users to send tweets. It contains a field for the content, a field for the topic and a little character count-down.
- TweetList.vue: This component uses the TweetCard.vue component to display not just one but multiple tweets.
- TweetSearch.vue: This component offers a reusable form to search for criteria. It is used on the topics page and the users page as we need to search for something on both of these pages.

### API

- fetch-tweets.js: Provides a function that returns all tweets from our program. In a future episode, we will transform that function slightly so it can filter through topics and users.
- get-tweet.js: Provides a function that returns a tweet account from a given public key.
- send-tweet.js: Provides a function that sends a SendTweet instruction to our program with all the required information.

### Composables (hooks for VueJS)

- useAutoresizeTextarea.js: This composable is used in the TweetForm.vue component and makes the “content” field automatically resize itself based on its content. That way the field contains only one line of text to start with but extends as the user types.
- useCountCharacterLimit.js: Also used by the TweetForm.vue component, this composable returns a reactive character count-down based on a given text and limit.
- useFromRoute.js: This composable is used by many components. It’s a little refactoring that helps deal with Vue Router hooks. Normally, we’d need to add some code for when we enter a router and some other code when the route updates but the components stay the same — e.g. the topic changes in the topics page. That function enables us to write some logic once that will be fired on both events.
- useSlug.js: This composable is used to transform any given text into a slug. For instance Solana is AWESOME will become solana-is-awesome. This is used anywhere we need to make sure the topic is provided as a slug. That way, we’ve got less risk of users tweeting on the same topic not finding each other’s tweets due to case sensitivity.

## Integrating with Solana wallets

### Install Solana wallet libraries

```
npm install solana-wallets-vue @solana/wallet-adapter-wallets
```

### Initialize the wallet store

- Phantom and Solflare to wallets
- initWallet: init global store, autoConnect whenever user refreshes page

### Use wallet UI components

```vue
// TheSidebar.vue
<wallet-multi-button></wallet-multi-button>
```

```ts
// main.js
import "solana-wallets-vue/styles.css";
import "./main.css";
// ...
```

### Update the design of the wallet button

- main.css

### Connect your wallet

- [install phantom](https://phantom.app/)

### Access wallet data

```ts
import { useWallet } from "solana-wallets-vue";
const data = useWallet();
```

- `wallet`: connected ? object with pubKey : null
- `ready`, `connected`, `connecting`, `disconnecting`: state boolean
- `select`, `connect`, `disconnect`: wallet UI component will do these
- `sendTransaction`, `signTransaction`, `signAllTransactions` and `signMessage`: sign messages and/or transactions

### Anchor wallet

- useWallet is not compatiable with anchor -> useAnchorWallet();

```ts
import { useAnchorWallet } from "solana-wallets-vue";
const wallet = useAnchorWallet();
```

### Reactive variables in VueJS

- `const state = ref(state)`: like useState(state)
- `state.value`: get state

### Use wallet data in components

- TweetForm should appear only if connected

```diff
# TweetForm.vue
+ import { useWallet } from 'solana-wallets-vue'
...
  // Permissions.
- const connected = ref(true) // TODO: Check connected wallet.
+ const { connected } = useWallet()
```

- Profile should appear only if connected

```ts
// TheSidebar.vue
import { WalletMultiButton, useWallet } from "solana-wallets-vue";
const { connected } = useWallet();
```

### Provide a workspace

```
touch src/composables/useWorkspace.js
```

- put static cluster address `http://127.0.0.1:8899`
- Connection + Wallet = Provider
- wallet state can be changed -> `wallet.value`
- access IDL file -> static dir for now

```
import idl from '../../../target/idl/solana_twitter.json'
```

- IDL + Provider = Program
- provider is also reactive state -> provider.value
- get programID from idl (should be after `anchor deploy`)
- `initWorkspace` at `App.vue`

### Use the workspace

- wallet.publicKey.toBase58()

## Fetching tweets in the frontend

### Fetching all tweets

```ts
// api/fetch-tweets.js
import { useWorkspace } from "@/composables";

export const fetchTweets = async () => {
  const { program } = useWorkspace();
  const tweets = await program.value.account.tweet.all();
  return tweets;
};
```

- But return type does not fit with privious mocked data

### The Tweet model

```
mkdir src/models
touch src/models/Tweet.js
touch src/models/index.js
```

```ts
import dayjs from "dayjs";

export class Tweet {
  constructor(publicKey, accountData) {
    this.publicKey = publicKey;
    this.author = accountData.author;
    this.timestamp = accountData.timestamp.toString();
    this.topic = accountData.topic;
    this.content = accountData.content;
  }

  get key() {
    return this.publicKey.toBase58();
  }

  get author_display() {
    const author = this.author.toBase58();
    return author.slice(0, 4) + ".." + author.slice(-4);
  }

  get created_at() {
    return dayjs.unix(this.timestamp).format("lll");
  }

  get created_ago() {
    return dayjs.unix(this.timestamp).fromNow();
  }
}
```

- args(publicKey, accountDate) -> assign it to each properties
- `key`: unique id that represents each tweet
- `author_display`: condensed version of pubKey to display author
- `created_at`, `created_ago`: human readable timestamp

```
npm install dayjs
```

- localize date format

```ts
// main.js
import dayjs from "dayjs";
import localizedFormat from "dayjs/plugin/localizedFormat";
import relativeTime from "dayjs/plugin/relativeTime";
dayjs.extend(localizedFormat);
dayjs.extend(relativeTime);
```

- Returning Tweet models

```ts
// fetch-tweets.js
return tweets.map(tweet => new Tweet(tweet.publicKey, tweet.account));
```

### Add links in the tweet card

- author's address -> author's page
- tweet's time -> show only that tweet
- tweet's topic -> topic's page

#### The author’s link

- me => profile, other => userinfo page

```ts
// TweetCard.vue
const authorRoute = computed(() => {
  if (
    wallet.value &&
    wallet.value.publicKey.toBase58() === tweet.value.author.toBase58()
  ) {
    return { name: "Profile" };
  } else {
    return { name: "Users", params: { author: tweet.value.author.toBase58() } };
  }
});
...
<router-link :to="authorRoute" class="hover:underline">
```

#### The tweet’s link

```ts
// TweetCard.vue
<router-link :to="{ name: 'Tweet', params: { tweet: tweet.publicKey.toBase58() } }" class="hover:underline">
```

#### The topic's link

```ts
<router-link v-if="tweet.topic" :to="{ name: 'Topics', params: { topic: tweet.topic } }" class="inline-block mt-2 text-pink-500 hover:underline">
```

### Supporting filters

- Create custom `topicFilter`, `authorFilter` at `fetch-tweets.js`

#### Fetching tweets by topic

- Add `topicFilter` at `PageTopics.vue`

#### Fetching tweets by author

- Add `authorFilter` at `PageUsers.vue`, `PageProfile.vue`

#### Fetching only one tweet

- Getting a tweet uses `fetch(pulicKey)`
- Create getTweet at `get-tweet.js`
