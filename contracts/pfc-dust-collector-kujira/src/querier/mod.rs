#[cfg(test)]
pub mod qry {
    use crate::contract::query;
    use cosmwasm_std::testing::mock_env;
    use cosmwasm_std::{from_json, Deps};
    use pfc_dust_collector_kujira::dust_collector::QueryMsg;
    use serde::de::DeserializeOwned;

    pub(crate) fn query_helper<T: DeserializeOwned>(deps: Deps, msg: QueryMsg) -> T {
        let bin = query(deps, mock_env(), msg).unwrap();
        //eprintln!("Query Response {:?}",&bin);
        from_json(bin).unwrap()
    }
}
