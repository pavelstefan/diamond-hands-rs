#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, Clone)]
pub struct Deposit {
    pub timestamp: u64,
    pub amount: u64,
}

#[elrond_wasm::contract]
pub trait DiamonHands {
    #[init]
    fn init(
        &self,
        duration_in_seconds: u64,
        #[var_args] opt_token_id: OptionalArg<TokenIdentifier>,
    ) -> SCResult<()> {
        require!(
            duration_in_seconds > 0,
            "Duration in seconds cannot be set to zero"
        );
        self.duration_in_seconds().set(&duration_in_seconds);

        let token_id = opt_token_id
            .into_option()
            .unwrap_or_else(TokenIdentifier::egld);
        require!(
            token_id.is_egld() || token_id.is_valid_esdt_identifier(),
            "Invalid token provided"
        );

        Ok(())
    }

    // endpoints

    #[payable("*")]
    #[endpoint]
    fn deposit(
        &self,
        #[payment_token] payment_token: TokenIdentifier,
        #[payment_amount] payment_amount: Self::BigUint,
    ) -> SCResult<()> {
        require!(
            payment_token == self.accepted_payment_token_id().get(),
            "Invalid payment token"
        );

        let caller = self.blockchain().get_caller();

        let current_block_timestamp = self.blockchain().get_block_timestamp();

        let new_deposit = Deposit {
            timestamp: current_block_timestamp,
            amount: payment_amount.to_u64().unwrap() as u64,
        };

        let mut user_deposits = Vec::new();

        if self.did_user_deposit(&caller) {
            user_deposits = self.user_deposit(&caller).get();
        }

        user_deposits.push(new_deposit);
        self.user_deposit(&caller).set(&user_deposits);

        Ok(())
    }

    #[endpoint]
    fn withdraw(&self) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        require!(self.did_user_deposit(&caller), "Must deposit first");

        let current_timestamp = self.blockchain().get_block_timestamp();
        let duration_in_seconds = self.duration_in_seconds().get();

        let user_deposits = self.user_deposit(&caller).get();

        let mut amount: u64 = 0;
        let mut keep = Vec::new();

        for deposit in &user_deposits {
            if deposit.timestamp + duration_in_seconds >= current_timestamp {
                amount += deposit.amount;
            } else {
                keep.push(Deposit {
                    timestamp: deposit.timestamp,
                    amount: deposit.amount
                });
            }
        }

        require!(amount > 0, "No founds available to withdraw");

        let token_id = self.accepted_payment_token_id().get();

        self.user_deposit(&caller).set(&keep);
        

        self.send().direct(&caller, &token_id, 0, &Self::BigUint::from(amount), b"withdraw successful");

        Ok(())
    }


    // views

    #[view(didUserDeposit)]
    fn did_user_deposit(&self, address: &Address) -> bool {
        !self.user_deposit(address).is_empty()
    }

    // storage

    #[view(getAcceptedPaymentToken)]
    #[storage_mapper("acceptedPaymentTokenId")]
    fn accepted_payment_token_id(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;


    #[view(getDurationTimestamp)]
    #[storage_mapper("durationInSeconds")]
    fn duration_in_seconds(&self) -> SingleValueMapper<Self::Storage, u64>;

    #[view(getUserDeposit)]
    #[storage_mapper("userDeposit")]
    fn user_deposit(&self, address: &Address) -> SingleValueMapper<Self::Storage, Vec<Deposit>>;
}
