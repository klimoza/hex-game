use crate::*;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub enum FinishedStreams {
    None,
    First,
    Second,
    Both,
}

#[near_bindgen]
impl Contract {
    #[private]
    pub fn get_game_internal(&self, index: GameIndex) -> Game {
        require!(env::predecessor_account_id() == env::current_account_id());
        let game = self.games.get(&index).map(|x| x.game);
        game.unwrap()
    }

    #[private]
    pub fn make_move_internal(
        &mut self,
        index: GameIndex,
        move_type: MoveType,
        cell: Option<Cell>,
    ) -> Promise {
        require!(env::predecessor_account_id() == env::current_account_id());
        let mut game_with_data = self.games.get(&index).expect("Game doesn't exist.");
        if game_with_data.game.is_finished {
            return Self::ext(env::current_account_id()).get_game_internal(index);
        }
        let old_board = game_with_data.game.board.clone();

        game_with_data.make_move(move_type, cell);

        env::log_str("Old board:");
        old_board.debug_logs();

        env::log_str("New board:");
        game_with_data.game.board.debug_logs();

        self.games.insert(&index, &game_with_data);

        if game_with_data.game.is_finished {
            if game_with_data.game.turn % 2 == 1 {
                env::log_str("First player wins!");
            } else {
                env::log_str("Second player wins!");
            }
            if let Some(bid) = self.bids.get(&index) {
                bid.stop_streams()
                    .then(self.player_won(
                        &bid,
                        &game_with_data.game,
                        game_with_data.game.turn as u8 % 2,
                    ))
                    .then(Self::ext(env::current_account_id()).get_game_internal(index))
            } else {
                Self::ext(env::current_account_id()).get_game_internal(index)
            }
        } else {
            if let Some(bid) = self.bids.get(&index) {
                if game_with_data.game.turn % 2 == 1 {
                    start_stream(bid.stream_to_second_player)
                        .then(Self::ext(env::current_account_id()).get_game_internal(index))
                } else {
                    start_stream(bid.stream_to_first_player)
                        .then(Self::ext(env::current_account_id()).get_game_internal(index))
                }
            } else {
                Self::ext(env::current_account_id()).get_game_internal(index)
            }
        }
    }

    #[private]
    pub fn resolve_streams(
        &mut self,
        game_id: GameIndex,
        move_type: MoveType,
        cell: Option<Cell>,
    ) -> Promise {
        require!(env::predecessor_account_id() == env::current_account_id());
        require!(env::promise_results_count() == 1, "ERR_WRONG_RESULTS_COUNT");
        let (res, stream1, stream2) = match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(val) => {
                if let Ok(res) =
                    near_sdk::serde_json::from_slice::<(FinishedStreams, Stream, Stream)>(&val)
                {
                    res
                } else {
                    env::panic_str("ERR_WRONG_VAL_RECEIVED")
                }
            }
            PromiseResult::Failed => env::panic_str("ERR_CALL_FAILED"),
        };

        let bid = self.bids.get(&game_id).unwrap();
        let mut game_with_data = self.games.get(&game_id).unwrap();
        game_with_data.game.is_finished = true;
        self.games.insert(&game_id, &game_with_data);
        let game = game_with_data.game;

        match res {
            FinishedStreams::None => {
                Self::ext(env::current_account_id()).make_move_internal(game_id, move_type, cell)
            }
            FinishedStreams::First => {
                let bal = stream2.balance;
                stop_stream(stream2.id.into())
                    .then(Promise::new(game.first_player.clone()).transfer(bal + bid.bid))
            }
            FinishedStreams::Second => {
                let bal = stream1.balance;
                stop_stream(stream1.id.into())
                    .then(Promise::new(game.second_player.clone()).transfer(bal + bid.bid))
            }
            FinishedStreams::Both => Promise::new(game.first_player.clone())
                .transfer(bid.bid)
                .then(Promise::new(game.second_player.clone()).transfer(bid.bid)),
        }
    }

    #[private]
    pub fn parse_two_promise_streams(&mut self) -> Promise {
        require!(env::predecessor_account_id() == env::current_account_id());
        require!(env::promise_results_count() == 2, "ERR_WRONG_RESULTS_COUNT");
        let mut stream1 = match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(val) => {
                if let Ok(stream) = near_sdk::serde_json::from_slice::<Stream>(&val) {
                    stream
                } else {
                    env::panic_str("ERR_WRONG_VAL_RECEIVED")
                }
            }
            PromiseResult::Failed => env::panic_str("ERR_CALL_FAILED"),
        };
        let mut stream2 = match env::promise_result(1) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(val) => {
                if let Ok(stream) = near_sdk::serde_json::from_slice::<Stream>(&val) {
                    stream
                } else {
                    env::panic_str("ERR_WRONG_VAL_RECEIVED")
                }
            }
            PromiseResult::Failed => env::panic_str("ERR_CALL_FAILED"),
        };
        if stream1.status == StreamStatus::Active && stream2.status == StreamStatus::Active {
            stream1.status = StreamStatus::Paused;
            stream2.status = StreamStatus::Paused;
            (pause_stream(stream1.id.into()).and(pause_stream(stream2.id.into())))
                .then(Self::ext(env::current_account_id()).parse_two_streams(stream1, stream2))
        } else if stream1.status == StreamStatus::Active {
            stream1.status = StreamStatus::Paused;
            pause_stream(stream1.id.into())
                .then(Self::ext(env::current_account_id()).parse_two_streams(stream1, stream2))
        } else if stream2.status == StreamStatus::Active {
            stream2.status = StreamStatus::Paused;
            pause_stream(stream2.id.into())
                .then(Self::ext(env::current_account_id()).parse_two_streams(stream1, stream2))
        } else {
            Self::ext(env::current_account_id()).parse_two_streams(stream1, stream2)
        }
    }

    #[private]
    pub fn parse_two_streams(
        &mut self,
        stream1: Stream,
        stream2: Stream,
    ) -> (FinishedStreams, Stream, Stream) {
        require!(env::predecessor_account_id() == env::current_account_id());
        match (stream1.status.clone(), stream2.status.clone()) {
            (StreamStatus::Active, _) => unreachable!(),
            (_, StreamStatus::Active) => unreachable!(),
            (
                _,
                StreamStatus::Finished {
                    reason: StreamFinishReason::StoppedByOwner,
                },
            ) => unreachable!(),
            (
                StreamStatus::Finished {
                    reason: StreamFinishReason::StoppedByOwner,
                },
                _,
            ) => unreachable!(),
            (
                _,
                StreamStatus::Finished {
                    reason: StreamFinishReason::FinishedWhileTransferred,
                },
            ) => unreachable!(),
            (
                StreamStatus::Finished {
                    reason: StreamFinishReason::FinishedWhileTransferred,
                },
                _,
            ) => unreachable!(),
            (
                _,
                StreamStatus::Finished {
                    reason: StreamFinishReason::FinishedBecauseCannotBeExtended,
                },
            ) => unreachable!(),
            (
                StreamStatus::Finished {
                    reason: StreamFinishReason::FinishedBecauseCannotBeExtended,
                },
                _,
            ) => unreachable!(),
            (
                _,
                StreamStatus::Finished {
                    reason: StreamFinishReason::StoppedByReceiver,
                },
            ) => (FinishedStreams::None, stream1, stream2),
            (
                StreamStatus::Finished {
                    reason: StreamFinishReason::StoppedByReceiver,
                },
                _,
            ) => (FinishedStreams::None, stream1, stream2),
            (
                StreamStatus::Finished {
                    reason: StreamFinishReason::FinishedNaturally,
                },
                StreamStatus::Finished {
                    reason: StreamFinishReason::FinishedNaturally,
                },
            ) => (FinishedStreams::Both, stream1, stream2),
            (
                StreamStatus::Finished {
                    reason: StreamFinishReason::FinishedNaturally,
                },
                _,
            ) => (FinishedStreams::First, stream1, stream2),
            (
                _,
                StreamStatus::Finished {
                    reason: StreamFinishReason::FinishedNaturally,
                },
            ) => (FinishedStreams::Second, stream1, stream2),
            _ => (FinishedStreams::None, stream1, stream2),
        }
    }
}
