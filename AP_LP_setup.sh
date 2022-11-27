#!/usr/bin/env bash

export init='
{
  "create_pair": {
    "pair_type": {
      "stable": {}
    },
    "asset_infos": [
      {
        "token": {
          "contract_addr": "terra1xztnx8mm7dagn4ck3dgylaqucp6h6agw83pmyc29hnplq7355trs78fkcq"
        }
      },
      {
        "native_token": {
          "denom": "uluna"
        }
      }
    ],
    "init_params":"eyJhbXAiOjEwfQ=="
  }
}'
export factory="terra1z3y69xas85r7egusa0c7m5sam0yk97gsztqmh8f2cc6rr4s4anysudp7k0"
#terrad tx wasm exec $factory "${init}" --from testadmin --fees 30000uluna --gas 500000

# Pair
#export contract="terra1v2ycfsv427m28tn32gjllza4p6hpe65excyxgtuszkycp73fjams85598j"
export lp_contract="terra1v2ycfsv427m28tn32gjllza4p6hpe65excyxgtuszkycp73fjams85598j"
export lp_token="terra1mqmrh89e42yk6vy026mawymz879d4p829560krcql3e0ws23lassk6hzx4"
export steak_token="terra1xztnx8mm7dagn4ck3dgylaqucp6h6agw83pmyc29hnplq7355trs78fkcq"
export generator="terra1gc4d4v82vjgkz0ag28lrmlxx3tf6sq69tmaujjpe7jwmnqakkx0qm28j2l"
export factory_query='{
     "pair": {
       "asset_infos": [
           {
             "token": {
               "contract_addr": "terra1xztnx8mm7dagn4ck3dgylaqucp6h6agw83pmyc29hnplq7355trs78fkcq"
           }
           },
           {
             "native_token": {
               "denom": "uluna"
           }
         }
       ]
     }
   }'

terrad query wasm cs smart $factory "${factory_query}"

# allowance

terrad tx wasm exec $steak_token '
{  "increase_allowance": { "amount": "1000000",
    "spender": "terra1v2ycfsv427m28tn32gjllza4p6hpe65excyxgtuszkycp73fjams85598j" }}'  --from testadmin --fees 30000uluna


terrad tx wasm exec $lp_contract '
{
  "provide_liquidity": {
    "assets": [
      {
        "amount": "1000000",
        "info": {
          "token": {
            "contract_addr": "terra1xztnx8mm7dagn4ck3dgylaqucp6h6agw83pmyc29hnplq7355trs78fkcq"
          }
        }
      },
      {
        "amount": "1000000",
        "info": {
          "native_token": {
            "denom": "uluna"
          }
        }
      }
    ],
    "auto_stake": false,
    "slippage_tolerance": "0.02"
  }
}'  --from testadmin --fees 30000uluna --amount 10000000uluna --gas 1000000

terrad query wasm cs smart $generator '{"config":{}}'|jq -e .data.allowed_reward_proxies


export reward_contract="terra10jmdvgf5tk5j3yq8c8jynxzj2ghf73sjy8l6xr85zclqpmkeyhpq6q2e36"
let msg = {
   "set_allowed_reward_proxies": {
       "proxies": [
                    "terra1w25ygvt976nh657v06h0csn47a56x5nwzt50xlw4xu8spr88a08qhn99py",
                    "terra1y3pjn6g0awzpkme2nfp4nzu75ae6wuhdfztdn2pqju5tlzhkphjq5st2ts",
                    "terra10jmdvgf5tk5j3yq8c8jynxzj2ghf73sjy8l6xr85zclqpmkeyhpq6q2e36"
                  ]
   }
}

let binmsg='ewogICAic2V0X2FsbG93ZWRfcmV3YXJkX3Byb3hpZXMiOiB7CiAgICAgICAicHJveGllcyI6IFsKICAgICAgICAgICAgICAgICAgICAidGVycmExdzI1eWd2dDk3Nm5oNjU3djA2aDBjc240N2E1Nng1bnd6dDUweGx3NHh1OHNwcjg4YTA4cWhuOTlweSIsCiAgICAgICAgICAgICAgICAgICAgInRlcnJhMXkzcGpuNmcwYXd6cGttZTJuZnA0bnp1NzVhZTZ3dWhkZnp0ZG4ycHFqdTV0bHpoa3BoanE1c3QydHMiLAogICAgICAgICAgICAgICAgICAgICJ0ZXJyYTEwam1kdmdmNXRrNWozeXE4YzhqeW54emoyZ2hmNzNzank4bDZ4cjg1emNscXBta2V5aHBxNnEyZTM2IgogICAgICAgICAgICAgICAgICBdCiAgIH0KfQ'


let allow_proxies_msg = '{
   order: "1",
   msg: {
       wasm: {
           execute: {
               contract_addr: "terra1gc4d4v82vjgkz0ag28lrmlxx3tf6sq69tmaujjpe7jwmnqakkx0qm28j2l",
               msg: "ewogICAic2V0X2FsbG93ZWRfcmV3YXJkX3Byb3hpZXMiOiB7CiAgICAgICAicHJveGllcyI6IFsKICAgICAgICAgICAgICAgICAgICAidGVycmExdzI1eWd2dDk3Nm5oNjU3djA2aDBjc240N2E1Nng1bnd6dDUweGx3NHh1OHNwcjg4YTA4cWhuOTlweSIsCiAgICAgICAgICAgICAgICAgICAgInRlcnJhMXkzcGpuNmcwYXd6cGttZTJuZnA0bnp1NzVhZTZ3dWhkZnp0ZG4ycHFqdTV0bHpoa3BoanE1c3QydHMiLAogICAgICAgICAgICAgICAgICAgICJ0ZXJyYTEwam1kdmdmNXRrNWozeXE4YzhqeW54emoyZ2hmNzNzank4bDZ4cjg1emNscXBta2V5aHBxNnEyZTM2IgogICAgICAgICAgICAgICAgICBdCiAgIH0KfQ",
               funds: []
           }
       }
   }
}'
