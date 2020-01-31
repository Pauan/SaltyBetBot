FAQ
===

* **Q**: Why does the bot bet so low? It's only betting $4,100!

   **A**: The bot tries to win upsets, which means that ~87% of the time it will lose. But when it wins, it wins big. This is the optimal way of betting (as proven by years of analysis).

   However, because it is losing so often, if it bet a large amount then it would be very volatile and it would quickly run out of money. So it bets a small amount so that way it will slowly and steadily gain money over time.

   As you gain more money, it will slowly increase the bet amount, until it reaches a maximum of $32,000.

* **Q**: Why does the bot only bet $1 in exhibitions?

   **A**: It's not possible for bots to bet in exhibitions, because there just isn't enough information (the SaltyBet website does not tell the character names or the palettes, it only tells the team names).

   So as a compromise, it bets $1 in order to gain exp, since that's the best it can do.

* **Q**: Why does the bot go all-in in tournaments?

   **A**: During tournaments you get a separate money pool. Even if you lose all of your tournament money, you will not lose any of your matchmaking money. It is impossible to lose money in tournaments, you can only gain money. So the optimal strategy in tournaments is to all-in, because you have nothing to lose.

How to use
==========

Disclaimer: I accept no responsibility if you lose salt from running this bot.

Before you install this extension, make sure that you have [Git](https://git-scm.com/downloads) installed.

1. In a console, do `git clone https://github.com/Pauan/SaltyBetBot.git SaltyBetBot-master`

2. In Chrome, go to the [`chrome://extensions/`](chrome://extensions/) URL.

3. Make sure that "Developer mode" is turned on (it's in the upper-right corner).

4. Click on the "Load unpacked" button, then go into the `SaltyBetBot-master` folder which you created in step 1, then select the `static` folder and click OK.

5. If everything was done correctly, the extension should now be loaded in Chrome, congratulations!

6. You can now go to [mugen.saltybet.com](http://mugen.saltybet.com/) and it will start to bet automatically.

How to upgrade
==============

1. Make sure that all of the SaltyBet tabs are closed.

2. Click on the square "S" button in the upper-right corner, and then Export your match records (just in case something goes wrong with the upgrade process).

3. In a console, go into the `SaltyBetBot-master` folder and then do `git pull`.

4. In Chrome, go to the [`chrome://extensions/`](chrome://extensions/) URL.

5. Find "Salty Bet Bot" in the extensions list, and then click the reload button (it looks like a circular arrow).

6. Re-open the [mugen.saltybet.com](http://mugen.saltybet.com/) website.

How to build (for programmers only)
===================================

You will need to install [Rust](https://www.rust-lang.org/en-US/install.html), [Node.js](https://nodejs.org/en/download/), and [Yarn](https://yarnpkg.com/en/docs/install#windows-stable).

If you are on Windows, then you also need to install the [Visual Studio build tools](https://visualstudio.microsoft.com/thank-you-downloading-visual-studio/?sku=BuildTools&rel=16) (make sure to enable the "C++ build tools" option).

Then run these commands to setup things:

```
rustup install nightly
rustup override set nightly
yarn
```

You only need to run the above commands one time.

Now you can run this command to build the project:

```
yarn build
```

You need to re-run the above command whenever you make any changes.

Lastly, you can load the `static` folder into Chrome as usual.

Whenever you rebuild you need to reload the extension in Chrome (using the circular arrow).
