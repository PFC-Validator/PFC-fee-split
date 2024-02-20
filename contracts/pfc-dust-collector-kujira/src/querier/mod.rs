#[cfg(test)]
pub mod qry {
    use cosmwasm_std::{from_json, testing::mock_env, Deps};
    use pfc_dust_collector_kujira::dust_collector::QueryMsg;
    use serde::de::DeserializeOwned;

    use crate::contract::query;

    pub(crate) fn query_helper<T: DeserializeOwned>(deps: Deps, msg: QueryMsg) -> T {
        let bin = query(deps, mock_env(), msg).unwrap();
        //eprintln!("Query Response {:?}",&bin);
        from_json(bin).unwrap()
    }
}
