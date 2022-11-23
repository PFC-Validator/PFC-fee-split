use cosmwasm_std::Addr;

const TERRA_ADDRESS_LENGTH: usize = 44;

pub fn is_contract(address: &Addr) -> bool {
    address.to_string().len() > TERRA_ADDRESS_LENGTH
}
