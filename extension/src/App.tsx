import { SettingsIcon } from "@chakra-ui/icons";
import { Box, Center, ChakraProvider, Flex, Heading, IconButton, Spacer } from "@chakra-ui/react";
import { faBug } from "@fortawesome/free-solid-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import * as React from "react";
import { useAsync } from "react-async";
import { browser } from "webextension-polyfill-ts";
import { getBalances, getLoanToSign, getOpenLoans, getSwapToSign, getWalletStatus } from "./background-proxy";
import AddressQr from "./components/AddressQr";
import WalletBalances from "./components/Balances";
import ConfirmLoanWizard from "./components/ConfirmLoanWizard";
import ConfirmSwap from "./components/ConfirmSwap";
import CreateWallet from "./components/CreateWallet";
import OpenLoans from "./components/OpenLoans";
import UnlockWallet from "./components/UnlockWallet";
import WithdrawAll from "./components/WithdrawAll";
import { Status } from "./models";
import theme from "./theme";

const App = () => {
    const walletStatusHook = useAsync({ promiseFn: getWalletStatus });
    const walletBalanceHook = useAsync({ promiseFn: getBalances });
    const swapToSignHook = useAsync({ promiseFn: getSwapToSign });
    const loanToSignHook = useAsync({ promiseFn: getLoanToSign });
    const openLoansHook = useAsync({ promiseFn: getOpenLoans });

    let { data: walletStatus, reload: reloadWalletStatus, error } = walletStatusHook;
    let { data: balanceUpdates, reload: reloadWalletBalances } = walletBalanceHook;
    let { data: swapToSign, reload: reloadSwapToSign } = swapToSignHook;
    let { data: loanToSign, reload: reloadLoanToSign } = loanToSignHook;
    let { data: openLoans, reload: reloadOpenLoans } = openLoansHook;

    const refreshAll = () => {
        reloadWalletBalances();
        reloadWalletStatus();
        reloadSwapToSign();
        reloadLoanToSign();
        reloadOpenLoans();
    };

    // we want to either sign a swap or the loan but not both:
    let signLoan = false;
    if (!swapToSign && loanToSign) {
        signLoan = true;
    }

    return (
        <ChakraProvider theme={theme}>
            <Box h={600} w={400}>
                {walletStatus?.status === Status.Loaded
                    && <>
                        <Flex>
                            <Center p="2">
                                <Heading size="md">Waves Wallet</Heading>
                            </Center>
                            <Spacer />
                            <Box>
                                <IconButton
                                    aria-label="Settings"
                                    icon={<SettingsIcon />}
                                    onClick={() => browser.runtime.openOptionsPage()}
                                />
                            </Box>
                        </Flex>

                        {balanceUpdates && <WalletBalances balanceUpdates={balanceUpdates} />}
                        {!signLoan && !swapToSign && <AddressQr />}
                        {!signLoan && !swapToSign && <WithdrawAll />}
                        {!signLoan && !swapToSign && <OpenLoans openLoans={openLoans} onRepayed={refreshAll} />}

                        {swapToSign && <ConfirmSwap
                            onCancel={refreshAll}
                            onSuccess={refreshAll}
                            swapToSign={swapToSign!}
                        />}
                        {signLoan
                            && <ConfirmLoanWizard
                                onCancel={refreshAll}
                                onSuccess={refreshAll}
                                loanToSign={loanToSign!}
                            />}
                    </>}
                {walletStatus?.status === Status.NotLoaded
                    && <>
                        <Heading>Unlock Wallet</Heading>
                        <UnlockWallet
                            onUnlock={refreshAll}
                        />
                    </>}
                {walletStatus?.status === Status.None
                    && <>
                        <Heading>Create Wallet</Heading>
                        <CreateWallet
                            onUnlock={refreshAll}
                        />
                    </>}
                {!walletStatus && error
                    && <Center>
                        Something is wrong. Can you catch the <FontAwesomeIcon size="7x" icon={faBug} />?
                        {error}
                    </Center>}
            </Box>
        </ChakraProvider>
    );
};

export default App;
