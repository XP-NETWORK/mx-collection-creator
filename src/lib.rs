#![no_std]

multiversx_sc::imports!();

#[multiversx_sc::contract]
pub trait CollectionCreator {
    #[view(collections)]
    #[storage_mapper("collections")]
    fn collections(&self, identifier: ManagedBuffer) -> SingleValueMapper<TokenIdentifier>;

    #[view(creators)]
    #[storage_mapper("creators")]
    fn creators(&self) -> UnorderedSetMapper<ManagedAddress>;

    #[init]
    fn init(&self, creators: ManagedVec<ManagedAddress>) {
        self.creators().extend(creators.into_iter());
    }

    #[payable("EGLD")]
    #[endpoint]
    fn create_collection(
        &self,
        identifier: ManagedBuffer,
        name: ManagedBuffer,
        ticker: ManagedBuffer,
        owner: ManagedAddress,
    ) {
        require!(
            self.creators().contains(&self.blockchain().get_caller()),
            "Only creators can create collections"
        );

        require!(
            self.collections(identifier.clone()).is_empty(),
            "Collection already exists"
        );

        let payment_amount = self.call_value().egld_value();

        self.send()
            .esdt_system_sc_proxy()
            .issue_non_fungible(
                payment_amount.clone_value(),
                &name,
                &ticker,
                NonFungibleTokenProperties {
                    can_change_owner: true,
                    can_freeze: true,
                    can_pause: true,
                    can_transfer_create_role: true,
                    can_upgrade: true,
                    can_wipe: true,
                    can_add_special_roles: true,
                },
            )
            .async_call()
            .with_callback(self.callbacks().esdt_issue_callback(identifier, owner))
            .call_and_exit();
    }

    #[callback]
    fn esdt_issue_callback(
        &self,
        identifier: ManagedBuffer,
        owner: ManagedAddress,
        #[call_result] result: ManagedAsyncCallResult<TokenIdentifier>,
    ) {
        match result {
            ManagedAsyncCallResult::Ok(tid) => {
                self.collections(identifier).set(tid.clone());
                self.send()
                    .esdt_system_sc_proxy()
                    .transfer_ownership(&tid, &owner)
                    .async_call()
                    .with_callback(self.callbacks().esdt_set_special_roles(owner, tid))
                    .call_and_exit();
            }
            ManagedAsyncCallResult::Err(err) => {
                panic!("Error issuing ESDT({}): {:?}", err.err_code, err.err_msg);
            }
        };
    }

    #[callback]
    fn esdt_set_special_roles(
        &self,
        owner: ManagedAddress,
        identifier: TokenIdentifier,
        #[call_result] result: ManagedAsyncCallResult<IgnoreValue>,
    ) {
        match result {
            ManagedAsyncCallResult::Ok(_) => self
                .send()
                .esdt_system_sc_proxy()
                .set_special_roles(
                    &owner,
                    &identifier,
                    [EsdtLocalRole::NftBurn, EsdtLocalRole::NftCreate]
                        .iter()
                        .map(|e| e.clone()),
                )
                .async_call()
                .with_callback(self.callbacks().after_set_roles_callback())
                .call_and_exit(),
            ManagedAsyncCallResult::Err(err) => {
                panic!(
                    "Error Transferring Ownership of ESDT({}): {:?}",
                    err.err_code, err.err_msg
                );
            }
        }
    }

    #[callback]
    fn after_set_roles_callback(&self, #[call_result] result: ManagedAsyncCallResult<IgnoreValue>) {
        match result {
            ManagedAsyncCallResult::Ok(_) => {}
            ManagedAsyncCallResult::Err(err) => {
                panic!(
                    "Error setting special roles of ESDT({}): {:?}",
                    err.err_code, err.err_msg
                );
            }
        }
    }
}
