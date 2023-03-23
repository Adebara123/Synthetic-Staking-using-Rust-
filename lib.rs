#![cfg_attr(not(feature = "std"), no_std)]
// use psp22::PSP22;

#[ink::contract]
pub mod staking {

    // =====================================
    //Library IMPORTED 
    // =====================================
    use openbrush::{
        contracts::{
            traits::psp22::PSP22Ref,
        },
    }; // this would be used for psp22 token interaction 
    use ink::{storage::Mapping};
    use ink::env::CallFlags;
    use ink::prelude::vec;


    // =========================================
    // ERROR DECLARATION 
    // =========================================
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        NotOwner,
        AddressIsAddressZero,
        AmountShouldBeGreaterThanZero,
        NotEnoughBalanceForReward,
        TokenTransferFailed,
        StakingStillInProgress
    }


    
    #[ink(storage)]
    pub struct Staking { 
        psp22_stake_token: ink::ContractRef<PSP22>,
        psp22_reward_token: ink::ContractRef<PSP22>,
        owner: AccountId,
        duration: Balance,
        finish_at: Balance,
        updated_at: Balance,
        reward_rate: Balance,
        reward_per_token_stored: Balance,
        total_supply: Balance,
        user_reward_per_token_paid: Mapping<AccountId, Balance>,
        rewards: Mapping<AccountId, Balance>,
        balance_of: Mapping<AccountId, Balance>,
    }

    impl Staking {
        
        
        // =========================================
        // Constructor
        // =========================================
        #[ink(constructor)]
        pub fn new(
            reward_duration: Balance,
            psp22_stake_token: ink::ContractRef<PSP22>,
            psp22_reward_token: ink::ContractRef<PSP22>,
        ) -> Self {
            Self { 
                psp22_stake_token,
                psp22_reward_token,
                owner: Self::env().caller(),
                duration: reward_duration,
                finish_at: 0,
                updated_at: 0,
                reward_rate: 0,
                reward_per_token_stored: 0,
                total_supply: 0,
                user_reward_per_token_paid: Mapping::default(),
                rewards: Mapping::default(),
                balance_of: Mapping::default(),
            }
        }

        // =========================================
        // Modifiers 
        // =========================================

        fn only_owner (&self) -> Result<(), Error> {
            if self.env().caller() == self.owner {
                Ok(())
            } else {
                Err(Error::NotOwner)
            }
        }

        fn update_reward (&mut self, address_acount: AccountId) {
            self.reward_per_token_stored = self.reward_per_token();
            self.updated_at = self.last_time_reward_applicable();

            if address_acount != self.zero_address() {
                self.rewards.insert(address_acount, &(self.earned(address_acount)));
                self.user_reward_per_token_paid.insert(address_acount, &(self.reward_per_token_stored));
            }

        }

        // fn address_zero_checker (&self) -> Result<(), Error> {
        //     if self.env().caller() == self.zero_address() {
        //         Err(Error::AddressIsAddressZero)
        //     }else {
        //         Ok(())
        //     }
        // }

        fn zero_address(&self) -> AccountId {
            [0u8; 32].into()
        }

        // =========================================
        // Write functions
        // =========================================
        /// Function is used to the reward duration
        #[ink(message)]
        pub fn set_rewards_duration(&mut self, reward_duration: Balance) -> Result<(), Error>{
            self.only_owner()?;
            if self.env().block_timestamp() as u128 >= self.finish_at {
             return   Err(Error::StakingStillInProgress)
            }
            self.duration += reward_duration;

            Ok(())
        }

       
        #[ink(message)]
        pub fn reward_per_token(&self) -> Balance {
            let result = if self.total_supply == 0 {
                self.reward_per_token_stored
              
            }else {
                self.reward_per_token_stored + 
                (self.reward_rate * (self.last_time_reward_applicable() - self.updated_at) * 1e18 as u128) /
                self.total_supply
            };
            result
        }


        /// This function is called by the user to stake into the contract 
        #[ink(message)]
        pub fn stake(&mut self, stake_amount: Balance) -> Result<(), Error>{
            self.update_reward(self.env().caller());
            if stake_amount <= 0 {
              return  Err(Error::AmountShouldBeGreaterThanZero)
            } 
            let caller = self.env().caller();
            // Transfer the token into the contract
            self.psp22_stake_token.transfer_from(caller, self.env().account_id(), &stake_amount);
            let curent_bal = self.balance_of.get(self.env().caller()).unwrap_or(0) + &stake_amount;
            self.balance_of.insert(self.env().caller(), &curent_bal);
            self.total_supply += &stake_amount;
            Ok(())
        }

        #[ink(message)]
        pub fn withdraw(&self, _amount: Balance) -> Result<(), Error> {
            self.update_reward(self.env().caller());
            if _amount < 0 {
                return Err(Error::AmountShouldBeGreaterThanZero)
            }
            let caller = self.env().caller();
            let new_bal = self.balance_of.get(self.env().caller()).unwrap_or(0) - &_amount;
            self.balance_of.insert(self.env().caller(), &new_bal);
            self.total_supply -= &_amount;
            // Transfer the token to the person
            self.psp22_stake_token.transfer_from(self.env().account_id(), caller, _amount);
            Ok(())
        }

        #[ink(message)]
        pub fn get_reward(&self) -> Result<(), Error> {
            self.update_reward(self.env().caller());
            let caller = self.env().caller();
            let reward = self.rewards.get(self.env().caller()).unwrap_or(0);
            if &reward > 0 {
                self.get_result.insert(self.env().caller(), 0);
                // Transfer the reward to the person 
                self.psp22_reward_token.transfer_from(self.env().account_id(), caller, reward);
            }
            Ok(())
        }

        #[ink(message)]
        pub fn update_reward_rate(&self, _amount: Balance) -> Result<(), Error> {
            self.only_owner()?;
            self.update_reward(self.zero_address());

            let caller = self.env().caller();
            self.psp22_reward_token.transfer_from(caller, self.env().account_id(), &_amount);

            if self.env().block_timestamp() as u128 >= self.finish_at {
                self.reward_rate = &_amount / self.duration;
            }else {
                let remaining_reward = (self.finish_at - self.env().block_timestamp() as u128) * self.reward_rate;
                self.reward_rate = (&_amount + self.remaining_reward) / self.duration;
            }

            if self.reward_rate < 0 {
                return Err(Error::)
            }
            if self.reward_rate * self.duration >= self.psp22_reward_token.balance_of(self.env().account_id()) as u128 {
                return Err(Error::NotEnoughBalanceForReward)
            }

            self.finish_at = self.env().block_timestamp() as u128 + self.duration();
            self.updated_at = self.env().block_timestamp() as u128;

            Ok(())
        } 




        // =========================================
        // View functions  
        // =========================================

        #[ink(message)]
        pub fn last_time_reward_applicable (&self) -> Balance {
            self.min(self.finish_at, self.env().block_timestamp() as u128)
        }

        #[ink(message)]
        pub fn earned (&self, address_account: AccountId) -> Balance {
            (self.balance_of.get(address_account).unwrap_or(0) * 
                (self.reward_per_token() - self.user_reward_per_token_paid.get(address_account).unwrap_or(0)) / 1e18 as u128) + 
                self.rewards.get(address_account).unwrap_or(0)
        }



        fn min (&self, x: Balance, y: Balance) -> Balance {
            if x <= y {
                x
            } else {
                y
            }
        }
    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    /// module and test functions are marked with a `#[test]` attribute.
    /// The below code is technically just normal Rust code.
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        // We test a simple use case of our contract.
        // #[ink::test]
        // fn it_works() {
        //     let mut staking = Staking::new(false);
        //     assert_eq!(staking.get(), false);
        //     staking.flip();
        //     assert_eq!(staking.get(), true);
        // }
    }


    /// This is how you'd write end-to-end (E2E) or integration tests for ink! contracts.
    ///
    /// When running these you need to make sure that you:
    /// - Compile the tests with the `e2e-tests` feature flag enabled (`--features e2e-tests`)
    /// - Are running a Substrate node which contains `pallet-contracts` in the background
    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        /// A helper function used for calling contract messages.
        use ink_e2e::build_message;

        /// The End-to-End test `Result` type.
        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        /// We test that we can upload and instantiate the contract using its default constructor.
        #[ink_e2e::test]
        async fn default_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            // Given
            let constructor = StakingRef::default();

            // When
            let contract_account_id = client
                .instantiate("staking", &ink_e2e::alice(), constructor, 0, None)
                .await
                .expect("instantiate failed")
                .account_id;

            // Then
            let get = build_message::<StakingRef>(contract_account_id.clone())
                .call(|staking| staking.get());
            let get_result = client.call_dry_run(&ink_e2e::alice(), &get, 0, None).await;
            assert!(matches!(get_result.return_value(), false));

            Ok(())
        }

        /// We test that we can read and write a value from the on-chain contract contract.
        #[ink_e2e::test]
        async fn it_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            // Given
            let constructor = StakingRef::new(false);
            let contract_account_id = client
                .instantiate("staking", &ink_e2e::bob(), constructor, 0, None)
                .await
                .expect("instantiate failed")
                .account_id;

            let get = build_message::<StakingRef>(contract_account_id.clone())
                .call(|staking| staking.get());
            let get_result = client.call_dry_run(&ink_e2e::bob(), &get, 0, None).await;
            assert!(matches!(get_result.return_value(), false));

            // When
            let flip = build_message::<StakingRef>(contract_account_id.clone())
                .call(|staking| staking.flip());
            let _flip_result = client
                .call(&ink_e2e::bob(), flip, 0, None)
                .await
                .expect("flip failed");

            // Then
            let get = build_message::<StakingRef>(contract_account_id.clone())
                .call(|staking| staking.get());
            let get_result = client.call_dry_run(&ink_e2e::bob(), &get, 0, None).await;
            assert!(matches!(get_result.return_value(), true));

            Ok(())
        }
    }
}
