How to use
==========

1. Download [this zip file](https://github.com/Pauan/SaltyBetBot/archive/master.zip) and then unzip it somewhere.

   (If you prefer, you can instead use `git clone https://github.com/Pauan/SaltyBetBot.git SaltyBetBot-master`)

2. In Chrome, go to the [`chrome://extensions/`](chrome://extensions/) URL.

3. Make sure that "Developer mode" is turned on (it's in the upper-right corner).

4. Click on the "Load unpacked" button, then go into the `SaltyBetBot-master` folder which you created in step 1, then select the `static` folder and click OK.

5. If everything was done correctly, the extension should now be loaded in Chrome, congratulations!

6. In the upper-right corner there is a new square "S" button. Click on it. This will cause a popup to appear. Click on the "Import" button.

7. Find the `SaltyBetBot-master` folder, then select the `SaltyBet Records.json` file and click OK. This will take several seconds, wait for it to finish loading!

8. Everything is now setup, you can simply go to [saltybet.com](http://www.saltybet.com/) and it will start to bet automatically.

9. It will automatically open up a second twitch.tv tab, keep it open! It is necessary for *both* tabs to be open in order for the extension to work.

   If you accidentally close the twitch.tv tab, just refresh the saltybet.com tab and it will re-open it.


How to build (for programmers only)
===================================

```
cd compile
cargo run --release
```
