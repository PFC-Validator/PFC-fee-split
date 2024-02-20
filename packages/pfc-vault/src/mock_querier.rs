use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::marker::PhantomData;

//use crate::proxy::execute_msgs::SwapOperation;
use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_json, to_json_binary, Addr, AllBalanceResponse, Api, BankQuery, Binary, CanonicalAddr,
    Coin, ContractResult, CustomQuery, OwnedDeps, Querier, QuerierResult, QueryRequest,
    SystemError, SystemResult, Uint128, WasmQuery,
};
use cw20::{Cw20QueryMsg, TokenInfoResponse};

//use crate::proxy::query_msgs::QueryMsg::SimulateSwapOperations;
//use crate::proxy::query_msgs::SimulateSwapOperationsResponse;
//use crate::test_constants::VALKYRIE_PROXY;

pub type CustomDeps = OwnedDeps<MockStorage, MockApi, WasmMockQuerier>;

/// mock_dependencies is a drop-in replacement for cosmwasm_std::testing::mock_dependencies
/// this uses our CustomQuerier.
pub fn custom_deps() -> CustomDeps {
    let custom_querier = WasmMockQuerier::new(MockQuerier::new(&[(MOCK_CONTRACT_ADDR, &[])]));

    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: custom_querier,
        custom_query_type: PhantomData,
    }
}

pub struct WasmMockQuerier {
    base: MockQuerier<QueryWrapper>,
    token_querier: TokenQuerier,
    // astroport_router_querier: AstroportRouterQuerier,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct QueryWrapper {}

// implement custom query
impl CustomQuery for QueryWrapper {}

#[derive(Clone, Default)]
pub struct TokenQuerier {
    // this lets us iterate over all pairs that match the first string
    balances: HashMap<String, HashMap<String, Uint128>>,
}

impl TokenQuerier {
    pub fn new(balances: &[(&str, &[(&str, &Uint128)])]) -> Self {
        TokenQuerier {
            balances: balances_to_map(balances),
        }
    }
}

pub(crate) fn balances_to_map(
    balances: &[(&str, &[(&str, &Uint128)])],
) -> HashMap<String, HashMap<String, Uint128>> {
    let mut balances_map: HashMap<String, HashMap<String, Uint128>> = HashMap::new();
    for (contract_addr, balances) in balances.iter() {
        let mut contract_balances_map: HashMap<String, Uint128> = HashMap::new();
        for (addr, balance) in balances.iter() {
            contract_balances_map.insert(addr.to_string(), **balance);
        }

        balances_map.insert(contract_addr.to_string(), contract_balances_map);
    }
    balances_map
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely here
        let request: QueryRequest<QueryWrapper> = match from_json(bin_request) {
            Ok(v) => v,
            Err(_) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: "Parsing query request".to_string(),
                    request: bin_request.into(),
                });
            }
        };
        self.handle_query(&request)
    }
}

impl WasmMockQuerier {
    pub fn handle_query(&self, request: &QueryRequest<QueryWrapper>) -> QuerierResult {
        match &request {
            QueryRequest::Wasm(WasmQuery::Raw { contract_addr, key }) => {
                self.handle_wasm_raw(contract_addr, key)
            }
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
                self.handle_wasm_smart(contract_addr, msg)
            }
            _ => self.base.handle_query(request),
        }
    }

    fn handle_wasm_raw(&self, contract_addr: &String, key: &Binary) -> QuerierResult {
        let key: &[u8] = key.as_slice();

        let mut result = self.query_token_info(contract_addr, key);

        if result.is_none() {
            result = self.query_balance(contract_addr, key);
        }

        if result.is_none() {
            return QuerierResult::Err(SystemError::UnsupportedRequest {
                kind: "handle_wasm_raw".to_string(),
            });
        }

        result.unwrap()
    }

    fn handle_wasm_smart(&self, contract_addr: &String, msg: &Binary) -> QuerierResult {
        let result = self.handle_cw20(contract_addr, msg);

        if result.is_none() {
            return QuerierResult::Err(SystemError::UnsupportedRequest {
                kind: "handle_wasm_smart".to_string(),
            });
        }

        result.unwrap()
    }

    fn handle_cw20(&self, contract_addr: &String, msg: &Binary) -> Option<QuerierResult> {
        match from_json(msg) {
            Ok(Cw20QueryMsg::Balance { address }) => {
                let default = Uint128::zero();
                let balance = *self.token_querier.balances[contract_addr]
                    .get(address.as_str())
                    .unwrap_or(&default);

                Some(SystemResult::Ok(ContractResult::from(to_json_binary(
                    &cw20::BalanceResponse { balance },
                ))))
            }
            Ok(_) => Some(QuerierResult::Err(SystemError::UnsupportedRequest {
                kind: "handle_wasm_smart:cw20".to_string(),
            })),
            Err(_) => None,
        }
    }

    fn query_token_info(&self, contract_addr: &String, key: &[u8]) -> Option<QuerierResult> {
        if key.to_vec() != to_length_prefixed(b"token_info").to_vec() {
            return None;
        }

        let balances = self.token_querier.balances.get(contract_addr);

        if balances.is_none() {
            return Some(SystemResult::Err(SystemError::InvalidRequest {
                request: key.into(),
                error: format!("No balance info exists for the contract {}", contract_addr,),
            }));
        }

        let balances = balances.unwrap();

        let mut total_supply = Uint128::zero();

        for balance in balances {
            total_supply += *balance.1;
        }

        Some(SystemResult::Ok(ContractResult::Ok(
            to_json_binary(&TokenInfoResponse {
                name: format!("{}Token", contract_addr),
                symbol: "TOK".to_string(),
                decimals: 6,
                total_supply,
            })
            .unwrap(),
        )))
    }

    fn query_balance(&self, contract_addr: &String, key: &[u8]) -> Option<QuerierResult> {
        let prefix_balance = to_length_prefixed(b"balance").to_vec();
        // if key[..prefix_balance.len()].to_vec() == prefix_balance {}
        let _ = key[..prefix_balance.len()].to_vec() == prefix_balance;
        let balances = self.token_querier.balances.get(contract_addr);

        if balances.is_none() {
            return Some(SystemResult::Err(SystemError::InvalidRequest {
                request: key.into(),
                error: format!("No balance info exists for the contract {}", contract_addr,),
            }));
        }

        let balances = balances.unwrap();

        let key_address: &[u8] = &key[prefix_balance.len()..];
        let address_raw: CanonicalAddr = CanonicalAddr::from(key_address);
        let api = MockApi::default();
        let address = match api.addr_humanize(&address_raw) {
            Ok(v) => v.to_string(),
            Err(_) => {
                return Some(SystemResult::Err(SystemError::InvalidRequest {
                    error: "Parsing query request".to_string(),
                    request: key.into(),
                }));
            }
        };
        let balance = match balances.get(&address) {
            Some(v) => v,
            None => {
                return Some(SystemResult::Err(SystemError::InvalidRequest {
                    error: "Balance not found".to_string(),
                    request: key.into(),
                }));
            }
        };

        Some(SystemResult::Ok(ContractResult::Ok(
            to_json_binary(&balance).unwrap(),
        )))
    }
}

const ZERO: Uint128 = Uint128::zero();

impl WasmMockQuerier {
    pub fn new(base: MockQuerier<QueryWrapper>) -> Self {
        WasmMockQuerier {
            base,
            token_querier: TokenQuerier::default(),
            //       astroport_router_querier: AstroportRouterQuerier::default(),
        }
    }

    // configure the mint whitelist mock querier
    pub fn with_token_balances(&mut self, balances: &[(&str, &[(&str, &Uint128)])]) {
        self.token_querier = TokenQuerier::new(balances);
    }

    pub fn plus_token_balances(&mut self, balances: &[(&str, &[(&str, &Uint128)])]) {
        for (token_contract, balances) in balances.iter() {
            let token_contract = token_contract.to_string();

            if !self.token_querier.balances.contains_key(&token_contract) {
                self.token_querier
                    .balances
                    .insert(token_contract.clone(), HashMap::new());
            }
            let token_balances = self
                .token_querier
                .balances
                .get_mut(&token_contract)
                .unwrap();

            for (account, balance) in balances.iter() {
                let account = account.to_string();
                let account_balance = token_balances.get(&account).unwrap_or(&ZERO);
                let new_balance = *account_balance + *balance;
                token_balances.insert(account, new_balance);
            }
        }
    }

    pub fn minus_token_balances(&mut self, balances: &[(&str, &[(&str, &Uint128)])]) {
        for (token_contract, balances) in balances.iter() {
            let token_contract = token_contract.to_string();

            if !self.token_querier.balances.contains_key(&token_contract) {
                self.token_querier
                    .balances
                    .insert(token_contract.clone(), HashMap::new());
            }
            let token_balances = self
                .token_querier
                .balances
                .get_mut(&token_contract)
                .unwrap();

            for (account, balance) in balances.iter() {
                let account = account.to_string();
                let account_balance = token_balances.get(&account).unwrap_or(&ZERO);
                let new_balance = account_balance.checked_sub(**balance).unwrap();
                token_balances.insert(account, new_balance);
            }
        }
    }

    pub fn plus_native_balance(&mut self, address: &str, balances: Vec<Coin>) {
        let mut current_balances = from_json::<AllBalanceResponse>(
            &self
                .base
                .handle_query(&QueryRequest::Bank(BankQuery::AllBalances {
                    address: address.to_string(),
                }))
                .unwrap()
                .unwrap(),
        )
        .unwrap()
        .amount;

        for coin in balances.iter() {
            let current_balance = current_balances.iter_mut().find(|c| c.denom == coin.denom);

            if let Some(balance) = current_balance {
                balance.amount += coin.amount;
            } else {
                current_balances.push(coin.clone());
            }
        }

        self.base
            .update_balance(Addr::unchecked(address.to_string()), current_balances);
    }

    pub fn minus_native_balance(&mut self, address: &str, balances: Vec<Coin>) {
        let mut current_balances = from_json::<AllBalanceResponse>(
            &self
                .base
                .handle_query(&QueryRequest::Bank(BankQuery::AllBalances {
                    address: address.to_string(),
                }))
                .unwrap()
                .unwrap(),
        )
        .unwrap()
        .amount;

        for coin in balances.iter() {
            let current_balance = current_balances.iter_mut().find(|c| c.denom == coin.denom);

            if let Some(coin_balance) = current_balance {
                coin_balance.amount = coin_balance.amount.checked_sub(coin.amount).unwrap();
            } else {
                panic!("Insufficient balance");
            }
        }

        self.base
            .update_balance(Addr::unchecked(address.to_string()), current_balances);
    }

    pub fn with_balance(&mut self, balances: &[(&str, &[Coin])]) {
        for (addr, balance) in balances {
            self.base
                .update_balance(Addr::unchecked(addr.to_string()), balance.to_vec());
        }
    }
}

// Copy from cosmwasm-storage v0.14.1
fn to_length_prefixed(namespace: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(namespace.len() + 2);
    out.extend_from_slice(&encode_length(namespace));
    out.extend_from_slice(namespace);
    out
}

// Copy from cosmwasm-storage v0.14.1
fn encode_length(namespace: &[u8]) -> [u8; 2] {
    if namespace.len() > 0xFFFF {
        panic!("only supports namespaces up to length 0xFFFF")
    }
    let length_bytes = (namespace.len() as u32).to_be_bytes();
    [length_bytes[2], length_bytes[3]]
}

/*
fn swap_operation_to_string(operation: &SwapOperation) -> (String, String) {
    match operation {
        SwapOperation::Swap {
            offer_asset_info,
            ask_asset_info,
        } => (offer_asset_info.to_string(), ask_asset_info.to_string()),
    }
}
*/
