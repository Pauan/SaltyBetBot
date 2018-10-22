How to use
==========

Disclaimer: I accept no responsibility if you lose salt from running this bot.

Before you install this extension, make sure that you have [Git](https://git-scm.com/downloads) installed.

1. Use `git clone https://github.com/Pauan/SaltyBetBot.git SaltyBetBot-master`

2. In Chrome, go to the [`chrome://extensions/`](chrome://extensions/) URL.

3. Make sure that "Developer mode" is turned on (it's in the upper-right corner).

4. Click on the "Load unpacked" button, then go into the `SaltyBetBot-master` folder which you created in step 1, then select the `static` folder and click OK.

5. If everything was done correctly, the extension should now be loaded in Chrome, congratulations!

6. In the upper-right corner there is a new square "S" button. Click on it. This will cause a popup to appear. Click on the "Import" button.

7. Find the `SaltyBetBot-master` folder, then select the `SaltyBet Records.json` file and click OK. This will take several seconds, wait for it to finish loading!

8. Everything is now setup, you can simply go to [saltybet.com](http://www.saltybet.com/) and it will start to bet automatically.

9. It will automatically open up a second twitch.tv tab, keep it open! It is necessary for *both* tabs to be open in order for the extension to work.

   If you accidentally close the twitch.tv tab, just refresh the saltybet.com tab and it will re-open it.

How to upgrade
==============

1. Go into the `SaltyBetBot-master` folder and then do `git pull`.

2. Make sure that all of the SaltyBet and Twitch.tv tabs are closed.

3. In Chrome, go to the [`chrome://extensions/`](chrome://extensions/) URL.

4. Find "Salty Bet Bot" in the extensions list, and then click the reload button (it looks like a circular arrow).

5. In the upper-right corner click the square "S" button.

6. Export your match records (just in case something goes wrong with the import process), then import the `SaltyBet Records.json` file (this will merge the new data with your existing data).

7. Re-open the [saltybet.com](http://www.saltybet.com/) website.

How to build (for programmers only)
===================================

Make sure that you [have Rust installed](https://www.rust-lang.org/en-US/install.html), and then run these commands to setup things:

```
rustup install nightly
rustup target add wasm32-unknown-unknown
rustup run nightly cargo install cargo-web
```

You only need to run the above commands one time.

Now you can run these commands to build the project:

```
cd compile
cargo run --release
```
