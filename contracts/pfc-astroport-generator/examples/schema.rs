//use ap_valkyrie::MigrateMsg;
use astroport::generator_proxy::{ExecuteMsg, InstantiateMsg, QueryMsg};
use cosmwasm_schema::write_api;
use pfc_vault::EmptyMigrateMsg;
fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,
        migrate: EmptyMigrateMsg,
    }
}
