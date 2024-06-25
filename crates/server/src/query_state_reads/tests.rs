use essential_server_types::{SlotsRequest, StateReadRequestType};
use essential_transaction_storage::Transaction;
use essential_types::{
    convert::word_4_from_u8_32,
    intent::{Directive, Intent},
    solution::{Mutation, Solution, SolutionData},
};
use test_utils::empty::Empty;

use crate::test_utils::{deploy_contracts, deploy_intent};

use super::*;

#[tokio::test]
async fn test_inline_query_state_reads() {
    let (addr, storage) = deploy_intent(Intent::empty()).await;
    let addr_words = word_4_from_u8_32(addr.set.0);

    let read_key_0: Vec<u8> = essential_state_read_vm::asm::to_bytes(vec![
        essential_state_read_vm::asm::Stack::Push(1).into(),
        essential_state_read_vm::asm::StateSlots::AllocSlots.into(),
        essential_state_read_vm::asm::Stack::Push(addr_words[0]).into(),
        essential_state_read_vm::asm::Stack::Push(addr_words[1]).into(),
        essential_state_read_vm::asm::Stack::Push(addr_words[2]).into(),
        essential_state_read_vm::asm::Stack::Push(addr_words[3]).into(),
        essential_state_read_vm::asm::Stack::Push(0).into(),
        essential_state_read_vm::asm::Stack::Push(1).into(), // key length
        essential_state_read_vm::asm::Stack::Push(1).into(), // num values to read
        essential_state_read_vm::asm::Stack::Push(0).into(), // slot index
        essential_state_read_vm::asm::StateRead::KeyRangeExtern,
        essential_state_read_vm::asm::ControlFlow::Halt.into(),
    ])
    .collect();

    let read_key_1 = essential_state_read_vm::asm::to_bytes(vec![
        essential_state_read_vm::asm::Stack::Push(1).into(),
        essential_state_read_vm::asm::StateSlots::AllocSlots.into(),
        essential_state_read_vm::asm::Stack::Push(addr_words[0]).into(),
        essential_state_read_vm::asm::Stack::Push(addr_words[1]).into(),
        essential_state_read_vm::asm::Stack::Push(addr_words[2]).into(),
        essential_state_read_vm::asm::Stack::Push(addr_words[3]).into(),
        essential_state_read_vm::asm::Stack::Push(1).into(),
        essential_state_read_vm::asm::Stack::Push(1).into(), // key length
        essential_state_read_vm::asm::Stack::Push(1).into(), // num values to read
        essential_state_read_vm::asm::Stack::Push(0).into(), // slot index
        essential_state_read_vm::asm::StateRead::KeyRangeExtern,
        essential_state_read_vm::asm::ControlFlow::Halt.into(),
    ])
    .collect();
    let state_read = vec![read_key_0.clone(), read_key_1];

    let query = QueryStateReads::inline_empty(state_read, Default::default());

    let outcome = query_state_reads(storage.clone().transaction(), query.clone())
        .await
        .unwrap();

    let expect = QueryStateReadsOutput::All(
        [(
            addr.set.clone(),
            [(vec![0], vec![]), (vec![1], vec![])].into_iter().collect(),
        )]
        .into_iter()
        .collect(),
        Slots {
            pre: vec![vec![], vec![]],
            post: vec![vec![], vec![]],
        },
    );
    assert_eq!(outcome, expect);

    storage
        .update_state(&addr.set, &vec![0], vec![12])
        .await
        .unwrap();

    storage
        .update_state(&addr.set, &vec![1], vec![42])
        .await
        .unwrap();

    storage
        .update_state(&addr.set, &vec![12], vec![99])
        .await
        .unwrap();

    let outcome = query_state_reads(storage.clone().transaction(), query.clone())
        .await
        .unwrap();

    let expect = QueryStateReadsOutput::All(
        [(
            addr.set.clone(),
            [(vec![0], vec![12]), (vec![1], vec![42])]
                .into_iter()
                .collect(),
        )]
        .into_iter()
        .collect(),
        Slots {
            pre: vec![vec![12], vec![42]],
            post: vec![vec![12], vec![42]],
        },
    );
    assert_eq!(outcome, expect);

    let read_key_state_slot = essential_state_read_vm::asm::to_bytes(vec![
        essential_state_read_vm::asm::Stack::Push(1).into(),
        essential_state_read_vm::asm::StateSlots::AllocSlots.into(),
        essential_state_read_vm::asm::Stack::Push(addr_words[0]).into(),
        essential_state_read_vm::asm::Stack::Push(addr_words[1]).into(),
        essential_state_read_vm::asm::Stack::Push(addr_words[2]).into(),
        essential_state_read_vm::asm::Stack::Push(addr_words[3]).into(),
        essential_state_read_vm::asm::Stack::Push(0).into(),
        essential_state_read_vm::asm::Stack::Push(0).into(),
        essential_state_read_vm::asm::Access::State.into(),
        essential_state_read_vm::asm::Stack::Push(1).into(), // key length
        essential_state_read_vm::asm::Stack::Push(1).into(), // num values to read
        essential_state_read_vm::asm::Stack::Push(0).into(), // slot index
        essential_state_read_vm::asm::StateRead::KeyRangeExtern,
        essential_state_read_vm::asm::ControlFlow::Halt.into(),
    ])
    .collect();

    let state_read = vec![read_key_0, read_key_state_slot];

    let query = QueryStateReads::inline_empty(state_read.clone(), Default::default());

    let outcome = query_state_reads(storage.clone().transaction(), query)
        .await
        .unwrap();

    let expect = QueryStateReadsOutput::All(
        [(
            addr.set.clone(),
            [(vec![0], vec![12]), (vec![12], vec![99])]
                .into_iter()
                .collect(),
        )]
        .into_iter()
        .collect(),
        Slots {
            pre: vec![vec![12], vec![99]],
            post: vec![vec![12], vec![99]],
        },
    );
    assert_eq!(outcome, expect);

    let query = QueryStateReads::inline_empty(state_read.clone(), StateReadRequestType::Reads);

    let outcome = query_state_reads(storage.clone().transaction(), query)
        .await
        .unwrap();

    let expect = QueryStateReadsOutput::Reads(
        [(
            addr.set.clone(),
            [(vec![0], vec![12]), (vec![12], vec![99])]
                .into_iter()
                .collect(),
        )]
        .into_iter()
        .collect(),
    );
    assert_eq!(outcome, expect);

    let query = QueryStateReads::inline_empty(
        state_read.clone(),
        StateReadRequestType::Slots(SlotsRequest::All),
    );

    let outcome = query_state_reads(storage.clone().transaction(), query)
        .await
        .unwrap();

    let expect = QueryStateReadsOutput::Slots(Slots {
        pre: vec![vec![12], vec![99]],
        post: vec![vec![12], vec![99]],
    });
    assert_eq!(outcome, expect);

    let query = QueryStateReads::inline_empty(
        state_read.clone(),
        StateReadRequestType::Slots(SlotsRequest::Pre),
    );

    let outcome = query_state_reads(storage.clone().transaction(), query)
        .await
        .unwrap();

    let expect = QueryStateReadsOutput::Slots(Slots {
        pre: vec![vec![12], vec![99]],
        post: vec![],
    });
    assert_eq!(outcome, expect);

    let query = QueryStateReads::inline_empty(
        state_read.clone(),
        StateReadRequestType::Slots(SlotsRequest::Post),
    );

    let outcome = query_state_reads(storage.clone().transaction(), query)
        .await
        .unwrap();

    let expect = QueryStateReadsOutput::Slots(Slots {
        pre: vec![],
        post: vec![vec![12], vec![99]],
    });
    assert_eq!(outcome, expect);

    let query = QueryStateReads::inline_empty(
        state_read.clone(),
        StateReadRequestType::All(SlotsRequest::All),
    );

    let outcome = query_state_reads(storage.clone().transaction(), query)
        .await
        .unwrap();

    let expect = QueryStateReadsOutput::All(
        [(
            addr.set.clone(),
            [(vec![0], vec![12]), (vec![12], vec![99])]
                .into_iter()
                .collect(),
        )]
        .into_iter()
        .collect(),
        Slots {
            pre: vec![vec![12], vec![99]],
            post: vec![vec![12], vec![99]],
        },
    );
    assert_eq!(outcome, expect);

    let query = QueryStateReads::inline_empty(
        state_read.clone(),
        StateReadRequestType::All(SlotsRequest::Pre),
    );

    let outcome = query_state_reads(storage.clone().transaction(), query)
        .await
        .unwrap();

    let expect = QueryStateReadsOutput::All(
        [(
            addr.set.clone(),
            [(vec![0], vec![12]), (vec![12], vec![99])]
                .into_iter()
                .collect(),
        )]
        .into_iter()
        .collect(),
        Slots {
            pre: vec![vec![12], vec![99]],
            post: vec![],
        },
    );
    assert_eq!(outcome, expect);

    let query = QueryStateReads::inline_empty(
        state_read.clone(),
        StateReadRequestType::All(SlotsRequest::Post),
    );

    let outcome = query_state_reads(storage.clone().transaction(), query)
        .await
        .unwrap();

    let expect = QueryStateReadsOutput::All(
        [(
            addr.set.clone(),
            [(vec![0], vec![12]), (vec![12], vec![99])]
                .into_iter()
                .collect(),
        )]
        .into_iter()
        .collect(),
        Slots {
            pre: vec![],
            post: vec![vec![12], vec![99]],
        },
    );
    assert_eq!(outcome, expect);
}

#[tokio::test]
async fn test_from_solution_query_state_reads() {
    let read_key_this_trans_data: Vec<u8> = essential_state_read_vm::asm::to_bytes(vec![
        essential_state_read_vm::asm::Stack::Push(1).into(),
        essential_state_read_vm::asm::StateSlots::AllocSlots.into(),
        essential_state_read_vm::asm::Stack::Push(0).into(), // key
        essential_state_read_vm::asm::Stack::Push(1).into(), // key len
        essential_state_read_vm::asm::Stack::Push(0).into(), // pathway
        essential_state_read_vm::asm::Access::Transient.into(),
        essential_state_read_vm::asm::Stack::Push(1).into(), // key length
        essential_state_read_vm::asm::Stack::Push(1).into(), // num values to read
        essential_state_read_vm::asm::Stack::Push(0).into(), // slot index
        essential_state_read_vm::asm::StateRead::KeyRange,
        essential_state_read_vm::asm::ControlFlow::Halt.into(),
    ])
    .collect();
    let read_key_this_dec_var = essential_state_read_vm::asm::to_bytes(vec![
        essential_state_read_vm::asm::Stack::Push(1).into(),
        essential_state_read_vm::asm::StateSlots::AllocSlots.into(),
        essential_state_read_vm::asm::Stack::Push(0).into(),
        essential_state_read_vm::asm::Access::DecisionVar.into(),
        essential_state_read_vm::asm::Stack::Push(1).into(), // key length
        essential_state_read_vm::asm::Stack::Push(1).into(), // num values to read
        essential_state_read_vm::asm::Stack::Push(0).into(), // slot index
        essential_state_read_vm::asm::StateRead::KeyRange,
        essential_state_read_vm::asm::ControlFlow::Halt.into(),
    ])
    .collect();
    let read_key_this_pre_slot = essential_state_read_vm::asm::to_bytes(vec![
        essential_state_read_vm::asm::Stack::Push(1).into(),
        essential_state_read_vm::asm::StateSlots::AllocSlots.into(),
        // Check if state is empty
        essential_state_read_vm::asm::Stack::Push(3).into(), // jump dist
        essential_state_read_vm::asm::Stack::Push(0).into(), // slot
        essential_state_read_vm::asm::Stack::Push(0).into(), // delta
        essential_state_read_vm::asm::Access::StateLen.into(),
        essential_state_read_vm::asm::Stack::Push(0).into(), // empty
        essential_state_read_vm::asm::Pred::Eq.into(),
        essential_state_read_vm::asm::Pred::Not.into(),
        essential_state_read_vm::asm::TotalControlFlow::JumpForwardIf.into(),
        // Jump over dec var if state is not empty
        essential_state_read_vm::asm::Stack::Push(0).into(),
        essential_state_read_vm::asm::Access::DecisionVar.into(),
        // Read state
        essential_state_read_vm::asm::Stack::Push(0).into(), // slot
        essential_state_read_vm::asm::Stack::Push(0).into(), // delta
        essential_state_read_vm::asm::Access::State.into(),
        essential_state_read_vm::asm::Stack::Push(1).into(), // key length
        essential_state_read_vm::asm::Stack::Push(1).into(), // num values to read
        essential_state_read_vm::asm::Stack::Push(0).into(), // slot index
        essential_state_read_vm::asm::StateRead::KeyRange,
        essential_state_read_vm::asm::ControlFlow::Halt.into(),
    ])
    .collect();
    let read_key_this_post_slot = essential_state_read_vm::asm::to_bytes(vec![
        essential_state_read_vm::asm::Stack::Push(1).into(),
        essential_state_read_vm::asm::StateSlots::AllocSlots.into(),
        essential_state_read_vm::asm::Stack::Push(0).into(), // slot
        essential_state_read_vm::asm::Stack::Push(1).into(), // delta
        essential_state_read_vm::asm::Access::State.into(),
        essential_state_read_vm::asm::Stack::Push(1).into(), // key length
        essential_state_read_vm::asm::Stack::Push(1).into(), // num values to read
        essential_state_read_vm::asm::Stack::Push(0).into(), // slot index
        essential_state_read_vm::asm::StateRead::KeyRange,
        essential_state_read_vm::asm::ControlFlow::Halt.into(),
    ])
    .collect();
    let read_other_state = essential_state_read_vm::asm::to_bytes(vec![
        essential_state_read_vm::asm::Stack::Push(1).into(),
        essential_state_read_vm::asm::StateSlots::AllocSlots.into(),
        essential_state_read_vm::asm::Stack::Push(1).into(),
        essential_state_read_vm::asm::Stack::Push(0).into(),
        essential_state_read_vm::asm::Stack::Push(4).into(),
        essential_state_read_vm::asm::Access::DecisionVarRange.into(),
        essential_state_read_vm::asm::Stack::Push(88).into(), // key
        essential_state_read_vm::asm::Stack::Push(1).into(),  // key length
        essential_state_read_vm::asm::Stack::Push(1).into(),  // num values to read
        essential_state_read_vm::asm::Stack::Push(0).into(),  // slot index
        essential_state_read_vm::asm::StateRead::KeyRangeExtern,
        essential_state_read_vm::asm::ControlFlow::Halt.into(),
    ])
    .collect();
    let read_key_other_pre_slot = essential_state_read_vm::asm::to_bytes(vec![
        essential_state_read_vm::asm::Stack::Push(1).into(),
        essential_state_read_vm::asm::StateSlots::AllocSlots.into(),
        // Check if state is empty
        essential_state_read_vm::asm::Stack::Push(3).into(), // jump dist
        essential_state_read_vm::asm::Stack::Push(4).into(), // slot
        essential_state_read_vm::asm::Stack::Push(0).into(), // delta
        essential_state_read_vm::asm::Access::StateLen.into(),
        essential_state_read_vm::asm::Stack::Push(0).into(), // empty
        essential_state_read_vm::asm::Pred::Eq.into(),
        essential_state_read_vm::asm::Pred::Not.into(),
        essential_state_read_vm::asm::TotalControlFlow::JumpForwardIf.into(),
        // Jump over dec var if state is not empty
        essential_state_read_vm::asm::Stack::Push(0).into(),
        essential_state_read_vm::asm::Access::DecisionVar.into(),
        // Read state
        essential_state_read_vm::asm::Stack::Push(4).into(), // slot
        essential_state_read_vm::asm::Stack::Push(0).into(), // delta
        essential_state_read_vm::asm::Access::State.into(),
        essential_state_read_vm::asm::Stack::Push(1).into(), // key length
        essential_state_read_vm::asm::Stack::Push(1).into(), // num values to read
        essential_state_read_vm::asm::Stack::Push(0).into(), // slot index
        essential_state_read_vm::asm::StateRead::KeyRange,
        essential_state_read_vm::asm::ControlFlow::Halt.into(),
    ])
    .collect();
    let read_key_other_post_slot = essential_state_read_vm::asm::to_bytes(vec![
        essential_state_read_vm::asm::Stack::Push(1).into(),
        essential_state_read_vm::asm::StateSlots::AllocSlots.into(),
        essential_state_read_vm::asm::Stack::Push(4).into(), // slot
        essential_state_read_vm::asm::Stack::Push(1).into(), // delta
        essential_state_read_vm::asm::Access::State.into(),
        essential_state_read_vm::asm::Stack::Push(1).into(), // key length
        essential_state_read_vm::asm::Stack::Push(1).into(), // num values to read
        essential_state_read_vm::asm::Stack::Push(0).into(), // slot index
        essential_state_read_vm::asm::StateRead::KeyRange,
        essential_state_read_vm::asm::ControlFlow::Halt.into(),
    ])
    .collect();
    let read_key_other_trans_data = essential_state_read_vm::asm::to_bytes(vec![
        essential_state_read_vm::asm::Stack::Push(1).into(),
        essential_state_read_vm::asm::StateSlots::AllocSlots.into(),
        essential_state_read_vm::asm::Stack::Push(0).into(), // key
        essential_state_read_vm::asm::Stack::Push(1).into(), // key len
        essential_state_read_vm::asm::Stack::Push(1).into(), // pathway
        essential_state_read_vm::asm::Access::Transient.into(),
        essential_state_read_vm::asm::Stack::Push(1).into(), // key length
        essential_state_read_vm::asm::Stack::Push(1).into(), // num values to read
        essential_state_read_vm::asm::Stack::Push(0).into(), // slot index
        essential_state_read_vm::asm::StateRead::KeyRange,
        essential_state_read_vm::asm::ControlFlow::Halt.into(),
    ])
    .collect();
    let state_read = vec![
        read_key_this_trans_data,
        read_key_this_dec_var,
        read_key_this_pre_slot,
        read_key_this_post_slot,
        read_other_state,
        read_key_other_pre_slot,
        read_key_other_post_slot,
        read_key_other_trans_data,
    ];

    let intent = Intent {
        state_read,
        constraints: Default::default(),
        directive: Directive::Satisfy,
    };

    let (addr, storage) = deploy_contracts(vec![vec![intent.clone()], vec![Intent::empty()]]).await;

    let solution = Solution {
        data: vec![
            SolutionData {
                intent_to_solve: addr[0][0].clone(),
                decision_variables: vec![vec![22], word_4_from_u8_32(addr[1][0].set.0).to_vec()],
                state_mutations: vec![Mutation {
                    key: vec![99],
                    value: vec![24],
                }],
                transient_data: vec![Mutation {
                    key: vec![0],
                    value: vec![99],
                }],
            },
            SolutionData {
                intent_to_solve: addr[1][0].clone(),
                decision_variables: Default::default(),
                state_mutations: vec![Mutation {
                    key: vec![88],
                    value: vec![77],
                }],
                transient_data: vec![Mutation {
                    key: vec![0],
                    value: vec![44],
                }],
            },
        ],
    };

    storage
        .update_state(&addr[0][0].set, &vec![33], vec![222])
        .await
        .unwrap();
    storage
        .update_state(&addr[0][0].set, &vec![22], vec![333])
        .await
        .unwrap();
    storage
        .update_state(&addr[1][0].set, &vec![88], vec![444])
        .await
        .unwrap();
    storage
        .update_state(&addr[0][0].set, &vec![24], vec![555])
        .await
        .unwrap();
    storage
        .update_state(&addr[0][0].set, &vec![444], vec![9])
        .await
        .unwrap();
    storage
        .update_state(&addr[0][0].set, &vec![77], vec![8])
        .await
        .unwrap();
    storage
        .update_state(&addr[0][0].set, &vec![44], vec![7])
        .await
        .unwrap();

    let query = QueryStateReads::from_solution(solution, 0, &intent, Default::default());
    let outcome = query_state_reads(storage.clone().transaction(), query)
        .await
        .unwrap();

    let expect = QueryStateReadsOutput::All(
        [
            (
                addr[1][0].set.clone(),
                [(vec![88], vec![444])].into_iter().collect(),
            ),
            (
                addr[0][0].set.clone(),
                [
                    (vec![99], vec![]),
                    (vec![22], vec![333]),
                    (vec![22], vec![333]),
                    (vec![24], vec![555]),
                    (vec![444], vec![9]),
                    (vec![77], vec![8]),
                    (vec![44], vec![7]),
                ]
                .into_iter()
                .collect(),
            ),
        ]
        .into_iter()
        .collect(),
        Slots {
            pre: vec![
                vec![],
                vec![333],
                vec![333],
                vec![555],
                vec![444],
                vec![9],
                vec![8],
                vec![7],
            ],
            post: vec![
                vec![24],
                vec![333],
                vec![333],
                vec![555],
                vec![77],
                vec![9],
                vec![8],
                vec![7],
            ],
        },
    );
    assert_eq!(outcome, expect);
}
