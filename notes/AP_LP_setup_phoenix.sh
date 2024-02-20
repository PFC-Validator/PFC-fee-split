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
export factory="terra14x9fr055x5hvr48hzy2t4q7kvjvfttsvxusa4xsdcy702mnzsvuqprer8r"

# Pair
#export contract="terra1v2ycfsv427m28tn32gjllza4p6hpe65excyxgtuszkycp73fjams85598j"
export steak_token="terra17aj4ty4sz4yhgm08na8drc0v03v2jwr3waxcqrwhajj729zhl7zqnpc0ml"

export factory_query='{
     "pair": {
       "asset_infos": [
           {
             "token": {
               "contract_addr": "terra17aj4ty4sz4yhgm08na8drc0v03v2jwr3waxcqrwhajj729zhl7zqnpc0ml"
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
export pair="terra1h32epkd72x7st0wk49z35qlpsxf26pw4ydacs8acq6uka7hgshmq7z7vl9"
export lp_token="terra1h3z2zv6aw94fx5263dy6tgz6699kxmewlx3vrcu4jjrudg6xmtyqk6vt0u"
export lp_contract=$pair
export generator="terra1ksvlfex49desf4c452j6dewdjs6c48nafemetuwjyj6yexd7x3wqvwa7j9"

# provide liquidity to LP (not run)

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


#export reward_contract="terra10jmdvgf5tk5j3yq8c8jynxzj2ghf73sjy8l6xr85zclqpmkeyhpq6q2e36"
export astro_contract="terra12jvzm2cy33zspvp8asn7ns98jmyk489es2cy2j8k926mr2n7metqha430q"

export msg = '{
"set_allowed_reward_proxies": {
"proxies": [
  "terra14ewvq39vg23j0hcesecv6hkzkwkvrnuxzd5sddmry9lx6qrhaxcqjdx6er",
  "terra15yuq64lp74df0d5pdcmwzep80j0aa4hs3fktqyupz4a82ayvdw2s4rdykv",
  "terra12jvzm2cy33zspvp8asn7ns98jmyk489es2cy2j8k926mr2n7metqha430q"
]}}
'
# XXX
export binmsg1='ewoic2V0X2FsbG93ZWRfcmV3YXJkX3Byb3hpZXMiOiB7CiJwcm94aWVzIjogWwogICJ0ZXJyYTE0ZXd2cTM5dmcyM2owaGNlc2VjdjZoa3prd2t2cm51eHpkNXNkZG1yeTlseDZxcmhheGNxamR4NmVyIiwKICAidGVycmExNXl1cTY0bHA3NGRmMGQ1cGRjbXd6ZXA4MGowYWE0aHMzZmt0cXl1cHo0YTgyYXl2ZHcyczRyZHlrdiIsCiAgInRlcnJhMTJqdnptMmN5MzN6c3B2cDhhc243bnM5OGpteWs0ODllczJjeTJqOGs5MjZtcjJuN21ldHFoYTQzMHEiCl19fQ=='


export allow_proxies_msg = '{
   order: "1",
   "msg": {
       "wasm": {
           "execute": {
               "contract_addr": "terra1ksvlfex49desf4c452j6dewdjs6c48nafemetuwjyj6yexd7x3wqvwa7j9",
               "msg": "ewoic2V0X2FsbG93ZWRfcmV3YXJkX3Byb3hpZXMiOiB7CiJwcm94aWVzIjogWwogICJ0ZXJyYTE0ZXd2cTM5dmcyM2owaGNlc2VjdjZoa3prd2t2cm51eHpkNXNkZG1yeTlseDZxcmhheGNxamR4NmVyIiwKICAidGVycmExNXl1cTY0bHA3NGRmMGQ1cGRjbXd6ZXA4MGowYWE0aHMzZmt0cXl1cHo0YTgyYXl2ZHcyczRyZHlrdiIsCiAgInRlcnJhMTJqdnptMmN5MzN6c3B2cDhhc243bnM5OGpteWs0ODllczJjeTJqOGs5MjZtcjJuN21ldHFoYTQzMHEiCl19fQ==",
               "funds": []
           }
       }
   }
}'
export msg2 = '{
 "move_to_proxy": {
   "lp_token": "terra1h3z2zv6aw94fx5263dy6tgz6699kxmewlx3vrcu4jjrudg6xmtyqk6vt0u",
   "proxy": "terra12jvzm2cy33zspvp8asn7ns98jmyk489es2cy2j8k926mr2n7metqha430q"
}}
 '
export binmsg2='ewogIm1vdmVfdG9fcHJveHkiOiB7CiAgICJscF90b2tlbiI6ICJ0ZXJyYTFoM3oyenY2YXc5NGZ4NTI2M2R5NnRnejY2OTlreG1ld2x4M3ZyY3U0ampydWRnNnhtdHlxazZ2dDB1IiwKICAgInByb3h5IjogInRlcnJhMTJqdnptMmN5MzN6c3B2cDhhc243bnM5OGpteWs0ODllczJjeTJqOGs5MjZtcjJuN21ldHFoYTQzMHEiCn19'
export allow_proxies_msg = '{
   "order": "2",
   "msg": {
       "wasm": {
           "execute": {
               "contract_addr": "terra1ksvlfex49desf4c452j6dewdjs6c48nafemetuwjyj6yexd7x3wqvwa7j9",
               "msg": "ewogIm1vdmVfdG9fcHJveHkiOiB7CiAgICJscF90b2tlbiI6ICJ0ZXJyYTFoM3oyenY2YXc5NGZ4NTI2M2R5NnRnejY2OTlreG1ld2x4M3ZyY3U0ampydWRnNnhtdHlxazZ2dDB1IiwKICAgInByb3h5IjogInRlcnJhMTJqdnptMmN5MzN6c3B2cDhhc243bnM5OGpteWs0ODllczJjeTJqOGs5MjZtcjJuN21ldHFoYTQzMHEiCn19",
               "funds": []
           }
       }
   }
}'
export  proposal_msg = '{
  "submit_proposal": {
  "title": "bLuna Rewards",
  "description": "bluna/luna rewards",
  "link": null,
  "messages": [{
      "order": "1",
      "msg": {
          "wasm": {
              "execute": {
                  "contract_addr": "terra1ksvlfex49desf4c452j6dewdjs6c48nafemetuwjyj6yexd7x3wqvwa7j9",
                  "msg": "ewoic2V0X2FsbG93ZWRfcmV3YXJkX3Byb3hpZXMiOiB7CiJwcm94aWVzIjogWwogICJ0ZXJyYTE0ZXd2cTM5dmcyM2owaGNlc2VjdjZoa3prd2t2cm51eHpkNXNkZG1yeTlseDZxcmhheGNxamR4NmVyIiwKICAidGVycmExNXl1cTY0bHA3NGRmMGQ1cGRjbXd6ZXA4MGowYWE0aHMzZmt0cXl1cHo0YTgyYXl2ZHcyczRyZHlrdiIsCiAgInRlcnJhMTJqdnptMmN5MzN6c3B2cDhhc243bnM5OGpteWs0ODllczJjeTJqOGs5MjZtcjJuN21ldHFoYTQzMHEiCl19fQ==",
                  "funds": []
              }
          }
      }
   }, {
         "order": "2",
         "msg": {
             "wasm": {
                 "execute": {
                     "contract_addr": "terra1ksvlfex49desf4c452j6dewdjs6c48nafemetuwjyj6yexd7x3wqvwa7j9",
                     "msg": "ewogIm1vdmVfdG9fcHJveHkiOiB7CiAgICJscF90b2tlbiI6ICJ0ZXJyYTFoM3oyenY2YXc5NGZ4NTI2M2R5NnRnejY2OTlreG1ld2x4M3ZyY3U0ampydWRnNnhtdHlxazZ2dDB1IiwKICAgInByb3h5IjogInRlcnJhMTJqdnptMmN5MzN6c3B2cDhhc243bnM5OGpteWs0ODllczJjeTJqOGs5MjZtcjJuN21ldHFoYTQzMHEiCn19",
                     "funds": []
                 }
             }
         }
      }]
  }
}'

export check='{"check_messages":{
 "messages": [{
     "order": "1",
     "msg": {
         "wasm": {
             "execute": {
                 "contract_addr": "terra1ksvlfex49desf4c452j6dewdjs6c48nafemetuwjyj6yexd7x3wqvwa7j9",
                 "msg": "ewoic2V0X2FsbG93ZWRfcmV3YXJkX3Byb3hpZXMiOiB7CiJwcm94aWVzIjogWwogICJ0ZXJyYTE0ZXd2cTM5dmcyM2owaGNlc2VjdjZoa3prd2t2cm51eHpkNXNkZG1yeTlseDZxcmhheGNxamR4NmVyIiwKICAidGVycmExNXl1cTY0bHA3NGRmMGQ1cGRjbXd6ZXA4MGowYWE0aHMzZmt0cXl1cHo0YTgyYXl2ZHcyczRyZHlrdiIsCiAgInRlcnJhMTJqdnptMmN5MzN6c3B2cDhhc243bnM5OGpteWs0ODllczJjeTJqOGs5MjZtcjJuN21ldHFoYTQzMHEiCl19fQ==",
                 "funds": []
             }
         }
     }
  }, {
        "order": "2",
        "msg": {
            "wasm": {
                "execute": {
                    "contract_addr": "terra1ksvlfex49desf4c452j6dewdjs6c48nafemetuwjyj6yexd7x3wqvwa7j9",
                    "msg": "ewogIm1vdmVfdG9fcHJveHkiOiB7CiAgICJscF90b2tlbiI6ICJ0ZXJyYTFoM3oyenY2YXc5NGZ4NTI2M2R5NnRnejY2OTlreG1ld2x4M3ZyY3U0ampydWRnNnhtdHlxazZ2dDB1IiwKICAgInByb3h5IjogInRlcnJhMTJqdnptMmN5MzN6c3B2cDhhc243bnM5OGpteWs0ODllczJjeTJqOGs5MjZtcjJuN21ldHFoYTQzMHEiCn19",
                    "funds": []
                }
            }
        }
     }]
}}'


export bin_prop_msg=''
export proposal_contract='terra1k9j8rcyk87v5jvfla2m9wp200azegjz0eshl7n2pwv852a7ssceqsnn7pq'


export vote='{"cast_vote": { "proposal_id": 86, "vote": "for"  }}'