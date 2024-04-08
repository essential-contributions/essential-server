mod test_slots {
    use crate::validate::{
        slots::{MAX_DECISION_VARIABLES, MAX_NUM_STATE_SLOTS, MAX_STATE_LEN},
        Validate,
    };
    use essential_types::slots::{Slots, StateSlot};
    use test_utils::empty::Empty;

    #[test]
    #[should_panic(expected = "Too many decision variables")]
    fn test_fail_too_many_decision_variables() {
        let slots = Slots {
            decision_variables: MAX_DECISION_VARIABLES + 1,
            state: Default::default(),
        };
        slots.validate().unwrap();
    }

    #[test]
    #[should_panic(expected = "Too many state slots")]
    fn test_fail_too_many_state_slots() {
        let slots = Slots {
            decision_variables: Default::default(),
            state: (0..MAX_NUM_STATE_SLOTS + 1)
                .map(|_| StateSlot::empty())
                .collect(),
        };
        slots.validate().unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid slots state length")]
    fn test_fail_invalid_state_slots_length() {
        let slots = Slots {
            decision_variables: Default::default(),
            state: vec![StateSlot {
                index: u32::MAX,
                amount: 1,
                program_index: Default::default(),
            }],
        };
        slots.validate().unwrap();
    }

    #[test]
    #[should_panic(expected = "Slots state length too large")]
    fn test_fail_state_slots_length_too_large() {
        let slots = Slots {
            decision_variables: Default::default(),
            state: vec![StateSlot {
                index: Default::default(),
                amount: MAX_STATE_LEN as u32 + 1,
                program_index: Default::default(),
            }],
        };
        slots.validate().unwrap();
    }
}

mod test_intent {
    use crate::validate::{
        intent::{
            MAX_CONSTRAINTS, MAX_CONSTRAINT_SIZE, MAX_DIRECTIVE_SIZE, MAX_INTENTS, MAX_STATE_READS,
            MAX_STATE_READ_SIZE,
        },
        Validate,
    };
    use essential_types::intent::{Directive, Intent};
    use test_utils::{empty::Empty, sign_corrupted, sign_with_random_keypair};

    #[test]
    fn test_empty_intent() {
        let intent = Intent::empty();
        let intent = sign_with_random_keypair(vec![intent]);
        intent.validate().unwrap();
    }

    #[test]
    #[should_panic(expected = "Failed to verify signature")]
    fn test_fail_invalid_signature() {
        let intent = Intent::empty();
        let intent = sign_corrupted(vec![intent]);
        intent.validate().unwrap();
    }

    #[test]
    #[should_panic(expected = "Too many intents")]
    fn test_fail_too_many_intents() {
        let intent_set: Vec<Intent> = (0..MAX_INTENTS + 1).map(|_| Intent::empty()).collect();
        intent_set.validate().unwrap();
    }

    #[test]
    #[should_panic(expected = "Directive too large")]
    fn test_fail_directive_too_large() {
        let mut intent = Intent::empty();
        intent.directive = Directive::Maximize(vec![0; MAX_DIRECTIVE_SIZE + 1]);
        intent.validate().unwrap();
    }

    #[test]
    #[should_panic(expected = "Too many state reads")]
    fn test_fail_too_many_state_reads() {
        let mut intent = Intent::empty();
        intent.state_read = (0..MAX_STATE_READS + 1).map(|_| vec![]).collect();
        intent.validate().unwrap();
    }

    #[test]
    #[should_panic(expected = "State read too large")]
    fn test_fail_state_read_too_large() {
        let mut intent = Intent::empty();
        intent.state_read = vec![vec![0u8; MAX_STATE_READ_SIZE + 1]];
        intent.validate().unwrap();
    }

    #[test]
    #[should_panic(expected = "Too many constraints")]
    fn test_fail_too_many_constraints() {
        let mut intent = Intent::empty();
        intent.constraints = (0..MAX_CONSTRAINTS + 1).map(|_| vec![]).collect();
        intent.validate().unwrap();
    }

    #[test]
    #[should_panic(expected = "Constraint too large")]
    fn test_fail_constraint_too_large() {
        let mut intent = Intent::empty();
        intent.constraints = vec![vec![0u8; MAX_CONSTRAINT_SIZE + 1]];
        intent.validate().unwrap();
    }
}

mod test_solution {
    use crate::{
        tests::deploy_intent,
        validate::{
            slots::MAX_DECISION_VARIABLES,
            solution::{MAX_SOLUTION_DATA, MAX_STATE_MUTATIONS},
            Validate, ValidateWithStorage,
        },
    };
    use essential_types::{
        intent::Intent,
        solution::{
            DecisionVariable, DecisionVariableIndex, PartialSolution, PartialSolutionData,
            Solution, SolutionData, StateMutation,
        },
        ContentAddress, IntentAddress, Signed,
    };
    use memory_storage::MemoryStorage;
    use storage::Storage;
    use test_utils::{empty::Empty, sign_corrupted, sign_with_random_keypair};

    async fn deploy_partial_solution<S: Storage>(
        storage: &S,
        partial_solution: PartialSolution,
    ) -> ContentAddress {
        let partial_solution = sign_with_random_keypair(partial_solution);
        storage
            .insert_partial_solution_into_pool(partial_solution.clone())
            .await
            .unwrap();
        ContentAddress(utils::hash(&partial_solution.data))
    }

    #[test]
    fn test_empty_solution() {
        let solution = sign_with_random_keypair(Solution::empty());
        solution.validate().unwrap();
    }

    #[test]
    #[should_panic(expected = "Failed to verify signature")]
    fn test_fail_invalid_signature() {
        let solution = sign_corrupted(Solution::empty());
        solution.validate().unwrap();
    }

    #[test]
    #[should_panic(expected = "Too many solution data")]
    fn test_fail_too_many_solution_data() {
        let solution_data: Vec<SolutionData> = (0..MAX_SOLUTION_DATA + 1)
            .map(|_| SolutionData::empty())
            .collect();
        solution_data.validate().unwrap();
    }

    #[test]
    #[should_panic(expected = "Too many decision variables")]
    fn test_fail_too_many_decision_variables() {
        let mut solution_data = vec![SolutionData::empty()];
        solution_data[0].decision_variables =
            vec![DecisionVariable::empty(); (MAX_DECISION_VARIABLES + 1) as usize];
        solution_data.validate().unwrap();
    }

    #[test]
    #[should_panic(expected = "Too many state mutations")]
    fn test_fail_too_many_state_mutations() {
        let state_mutations: Vec<StateMutation> = (0..MAX_STATE_MUTATIONS + 1)
            .map(|_| StateMutation::empty())
            .collect();
        state_mutations.validate().unwrap();
    }

    #[test]
    #[should_panic(expected = "Too many partial solutions")]
    fn test_fail_too_many_partial_solutions() {
        let state_mutations: Vec<Signed<ContentAddress>> = (0..MAX_STATE_MUTATIONS + 1)
            .map(|_| sign_with_random_keypair(ContentAddress::empty()))
            .collect();
        state_mutations.validate().unwrap();
    }

    #[tokio::test]
    async fn test_retrieve_intent_set() {
        let storage = MemoryStorage::new();
        let solution = Solution::empty();
        let intent = Intent::empty();
        let address = deploy_intent(&storage, intent).await;
        let solution_data = vec![SolutionData {
            intent_to_solve: address.clone(),
            decision_variables: Default::default(),
        }];
        solution_data
            .validate_with_storage(&storage, solution)
            .await
            .unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "Failed to retrieve intent set from storage")]
    async fn test_fail_to_retrieve_intent_set() {
        let storage = MemoryStorage::new();
        let solution = Solution::empty();
        let mut solution_data = vec![SolutionData::empty()];
        solution_data[0].intent_to_solve = IntentAddress::empty();
        solution_data
            .validate_with_storage(&storage, solution)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_retrieve_partial_solution() {
        let storage = MemoryStorage::new();
        let intent = Intent::empty();
        let intent_address = deploy_intent(&storage, intent).await;
        let mut partial_solution = PartialSolution::empty();
        partial_solution.data = vec![PartialSolutionData {
            intent_to_solve: intent_address.clone(),
            decision_variables: vec![None],
        }];
        let partial_solution_address =
            deploy_partial_solution(&storage, partial_solution.clone()).await;
        let mut solution = Solution::empty();
        let partial_solutions = vec![sign_with_random_keypair(partial_solution_address)];
        solution.partial_solutions = partial_solutions.clone();

        partial_solutions
            .validate_with_storage(&storage, solution)
            .await
            .unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "Failed to retrieve partial solution from storage")]
    async fn test_fail_to_retrieve_partial_solution() {
        let storage = MemoryStorage::new();
        let solution = Solution::empty();
        let partial_solutions = vec![sign_with_random_keypair(ContentAddress::empty())];
        partial_solutions
            .validate_with_storage(&storage, solution)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_all_intents_must_be_in_the_set() {
        let storage = MemoryStorage::new();
        let solution = Solution::empty();
        let intent = Intent::empty();
        let intent_address = deploy_intent(&storage, intent).await;
        let solution_data = vec![SolutionData {
            intent_to_solve: intent_address.clone(),
            decision_variables: Default::default(),
        }];
        solution_data
            .validate_with_storage(&storage, solution)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_all_state_mutations_must_have_an_intent_in_the_set() {
        let storage = MemoryStorage::new();
        let intent = Intent::empty();
        let intent_address = deploy_intent(&storage, intent).await;
        let mut solution = Solution::empty();
        solution.state_mutations = vec![StateMutation {
            pathway: 0,
            mutations: Default::default(),
        }];
        solution.data = vec![SolutionData {
            intent_to_solve: intent_address.clone(),
            decision_variables: Default::default(),
        }];
        solution
            .clone()
            .data
            .validate_with_storage(&storage, solution)
            .await
            .unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "All state mutations must have an intent in the set")]
    async fn test_fail_all_state_mutations_must_have_an_intent_in_the_set() {
        let storage = MemoryStorage::new();
        let mut solution = Solution::empty();
        solution.state_mutations = vec![StateMutation {
            pathway: 0,
            mutations: Default::default(),
        }];
        solution
            .clone()
            .data
            .validate_with_storage(&storage, solution)
            .await
            .unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "Ensure all solution data is valid")]
    async fn test_fail_all_solution_data_is_valid() {
        let storage = MemoryStorage::new();
        let intent = Intent::empty();
        let intent_address = deploy_intent(&storage, intent).await;
        let mut solution = Solution::empty();
        solution.data = vec![SolutionData {
            intent_to_solve: intent_address.clone(),
            decision_variables: vec![DecisionVariable::Inline(0)],
        }];
        solution
            .clone()
            .data
            .validate_with_storage(&storage, solution)
            .await
            .unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "Invalid transient decision variable")]
    async fn test_fail_invalid_transient_decision_variable() {
        let storage = MemoryStorage::new();
        let mut intent = Intent::empty();
        intent.slots.decision_variables = 1;
        let intent_address = deploy_intent(&storage, intent).await;
        let mut solution = Solution::empty();
        solution.data = vec![SolutionData {
            intent_to_solve: intent_address.clone(),
            decision_variables: vec![DecisionVariable::Transient(DecisionVariableIndex {
                solution_data_index: 1, // TODO: does not fail when this is 0. Confirm this should not be the case
                variable_index: Default::default(),
            })],
        }];
        solution
            .clone()
            .data
            .validate_with_storage(&storage, solution)
            .await
            .unwrap();
    }
}
