use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg, Vote, VoteResponse};
use crate::state::{config, config_read, State, VoteInfo};
use cosmwasm_std::{
    to_binary, Api, Binary, Env, Extern, HandleResponse, HumanAddr, InitResponse, MessageInfo,
    Querier, StdResult, Storage,
};

use std::collections::HashMap;

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    _info: MessageInfo,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let state = State {
        start: msg.start,
        end: msg.end,
        candidates: msg.candidates,
        votes: Vec::new(),
    };
    config(&mut deps.storage).save(&state)?;

    Ok(InitResponse::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::Vote { candidate } => try_vote(deps, env, info, candidate),
    }
}

pub fn try_vote<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    info: MessageInfo,
    candidate: HumanAddr,
) -> Result<HandleResponse, ContractError> {
    config(&mut deps.storage).update(|mut state| -> Result<_, ContractError> {
        if env.block.height < state.start || env.block.height > state.end {
            return Err(ContractError::NotAllowance {
                begin: state.start,
                end: state.end,
            });
        }
        state.votes.push(VoteInfo {
            voter: info.sender,
            candidate: candidate,
        });
        Ok(state)
    })?;
    Ok(HandleResponse::default())
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    _env: Env,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetVoteInfo {} => to_binary(&query_vote_info(deps)?),
    }
}

fn query_vote_info<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<VoteResponse> {
    let state = config_read(&deps.storage).load()?;
    let mut vote_info = HashMap::new();
    for vote in state.votes {
        let count = vote_info.entry(vote.candidate).or_insert(0);
        *count += 1;
    }

    let mut votes = Vec::new();
    for (candidate, count) in vote_info {
        votes.push(Vote {
            candidate: candidate,
            count: count,
        });
    }
    Ok(VoteResponse { votes: votes, start: state.start, end: state.end })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InitMsg {
            start: 10,
            end: 100,
            candidates: Vec::new(),
        };
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = init(&mut deps, mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(&deps, mock_env(), QueryMsg::GetVoteInfo {}).unwrap();
        let value: VoteResponse = from_binary(&res).unwrap();
        assert_eq!(10, value.start);
        assert_eq!(100, value.end);
    }

    #[test]
    fn vote() {
        let mut deps = mock_dependencies(&coins(2, "token"));

        let mut candidates:Vec<HumanAddr> = Vec::new();
        candidates.push("candidates1".into());
        candidates.push("candidates2".into());
        let msg = InitMsg {
            start: 10_000,
            end: 20_000,
            candidates: Vec::new(),
        };
        let info = mock_info("creator", &coins(2, "token"));
        let _res = init(&mut deps, mock_env(), info, msg).unwrap();

        // beneficiary can release it
        let info = mock_info("voter1", &coins(2, "token"));
        let msg = HandleMsg::Vote {candidate:"candidates1".into()};
        let _res = handle(&mut deps, mock_env(), info, msg).unwrap();

        // should increase counter by 1
        let res = query(&deps, mock_env(), QueryMsg::GetVoteInfo {}).unwrap();
        let value: VoteResponse = from_binary(&res).unwrap();
        assert_eq!(10_000, value.start);
        assert_eq!(20_000, value.end);
        assert_eq!("candidates1", value.votes[0].candidate);
        assert_eq!(1, value.votes[0].count);
    }
}