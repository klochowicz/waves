import * as assert from "assert";
import fetch from "node-fetch";
import {Builder, By, until} from "selenium-webdriver";
import {main} from "ts-node/dist/bin";

const firefox = require("selenium-webdriver/firefox");
const firefoxPath = require("geckodriver").path;

const getElementById = async (driver, xpath, timeout = 2000) => {
    const el = await driver.wait(until.elementLocated(By.xpath(xpath)), timeout);
    return await driver.wait(until.elementIsVisible(el), timeout);
};

describe("webdriver", () => {
    let driver;
    let extensionId;
    // TODO: Do not hard-code website URL
    const webAppUrl = "http://localhost:3004";
    const webAppTitle = "Waves";
    const extensionTitle = "Waves Wallet";

    beforeAll(async () => {
        let service = new firefox.ServiceBuilder(firefoxPath);

        // const options = new firefox.Options().headless();
        const options = new firefox.Options();

        driver = new Builder()
            .setFirefoxService(service)
            .forBrowser("firefox")
            .setFirefoxOptions(options)
            .build();

        driver.getProfil;
        // this works :)
        await driver.installAddon("../extension/web-ext-artifacts/waves_wallet-0.0.1.zip", true);

        // this probably works forever unless we change something and then it won't work anymore
        await driver.get("about:debugging#/runtime/this-firefox");
        const extensionElement = await getElementById(
            driver,
            "//span[contains(text(),'waves_wallet')]//"
            + "parent::li/section/dl/div//dt[contains(text(),'Internal UUID')]/following-sibling::dd",
        );
        extensionId = await extensionElement.getText();

        // load webapp again
        await driver.get(webAppUrl);
        // Assert that webapp window is loaded
        await driver.wait(until.titleIs(webAppTitle), 10000);

        // Check we don't have other windows open already
        assert((await driver.getAllWindowHandles()).length === 1);

        // Opens a new tab and switches to new tab
        await driver.switchTo().newWindow('tab');

        // Open extension
        let extensionUrl = `moz-extension://${extensionId}/popup.html`;
        await driver.get(`${extensionUrl}`);

        // Assert that extension window is loaded
        await driver.wait(until.titleIs(extensionTitle), 10000);
    });

    afterAll(async () => {
        // await driver.quit();
    });

    async function getWindowHandle(name: string) {
        let allWindowHandles = await driver.getAllWindowHandles();
        for (const windowHandle of allWindowHandles) {
            await driver.switchTo().window(windowHandle)
            const title = await driver.getTitle();
            if (title === name) {
                return windowHandle;
            }
        }
    }

    async function switchToWindow(name: string) {
        await driver.switchTo().window(await getWindowHandle(name));
        console.log(`Switched to ${await driver.getTitle()}`);
    }

    test("sell swap", async () => {

        await switchToWindow(extensionTitle);

        let password = "foo";
        console.log("Setting password");
        let passwordInput = await getElementById(driver, "//input[@data-cy='create-wallet-password-input']");
        await passwordInput.sendKeys(password);

        console.log("Creating wallet");
        let createWalletButton = await getElementById(driver, "//button[@data-cy='create-wallet-button']");
        await createWalletButton.click();

        console.log("Getting address");
        let addressField = await getElementById(driver, "//p[@data-cy='wallet-address-text-field']");
        let address = await addressField.getText();
        console.log(`Address found: ${address}`);

        await fetch(`${webAppUrl}/api/faucet/${address}`, {
            method: "POST",
        });

        await driver.wait(
            async () => {
                while (true) {
                    try {
                        await driver.navigate().refresh();
                        let btcAmount = await getElementById(driver, "//p[@data-cy='L-BTC-balance-text-field']");
                        console.log("Found btc amount: " + btcAmount);
                        return true;
                    } catch (_) {
                        //ignore
                    }
                }
            },
            10000,
        );

        await switchToWindow(webAppTitle);
        await driver.navigate().refresh();
        console.log("Setting alpha input amount");
        let alphaAmountInput = await getElementById(driver, "//div[@data-cy='Alpha-amount-input']//input");
        await alphaAmountInput.clear();
        await alphaAmountInput.sendKeys("0.4");

        console.log("Checking if button is available.");

        let swapButton = await getElementById(driver, "//button[@data-cy='swap-button']");
        await driver.wait(
            async () => {
                let buttonEnabled = await swapButton.isEnabled();
                while (!buttonEnabled) {
                    console.log("Button is enabled = " + buttonEnabled);
                    try {
                        await driver.navigate().refresh();
                        await sleep(1000);
                        swapButton = await getElementById(driver, "//button[@data-cy='swap-button']");
                    } catch (_) {
                        //ignore
                    }
                }
                return true;
            },
            20000,
        );
        console.log("Swap button is available: " + await swapButton.isEnabled());
        await swapButton.click();

        // await driver.switchTo().window(extensionWindow);

        // await getElementById(driver, "//button[@data-cy='swap-button']");
    }, 20000);
});

export async function sleep(time: number) {
    return new Promise((res) => {
        setTimeout(res, time);
    });
}
