use crate::{game::Player, *};

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub enum FinishedStreams {
    None,
    First,
    Second,
    Both,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub enum StatusType {
    Active,
    Paused,
    Finished,
}

impl StatusType {
    pub fn new(status: StreamStatus) -> Self {
        match status {
            StreamStatus::Active => Self::Active,
            StreamStatus::Initialized => Self::Paused,
            StreamStatus::Paused => Self::Paused,
            StreamStatus::Finished {
                reason: StreamFinishReason::StoppedByReceiver,
            } => Self::Paused,
            StreamStatus::Finished {
                reason: StreamFinishReason::FinishedBecauseCannotBeExtended,
            } => Self::Finished,
            StreamStatus::Finished {
                reason: StreamFinishReason::FinishedNaturally,
            } => Self::Finished,
            StreamStatus::Finished {
                reason: StreamFinishReason::FinishedWhileTransferred,
            } => Self::Finished,
            StreamStatus::Finished {
                reason: StreamFinishReason::StoppedByOwner,
            } => Self::Finished,
        }
    }
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

        if game_with_data.game.is_finished {
            if game_with_data.game.turn % 2 == 1 {
                game_with_data.game.winner = Some(Player::First);
                env::log_str("First player wins!");
            } else {
                game_with_data.game.winner = Some(Player::Second);
                env::log_str("Second player wins!");
            }
            self.games.insert(&index, &game_with_data);
            let winner = game_with_data.game.winner.clone();
            if let Some(bid) = self.bids.get(&index) {
                bid.stop_streams()
                    .then(self.player_won(&bid, &game_with_data.game, winner.unwrap()))
                    .then(Self::ext(env::current_account_id()).get_game_internal(index))
            } else {
                Self::ext(env::current_account_id()).get_game_internal(index)
            }
        } else {
            self.games.insert(&index, &game_with_data);
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

        match res {
            FinishedStreams::None => {
                Self::ext(env::current_account_id()).make_move_internal(game_id, move_type, cell)
            }
            FinishedStreams::First => {
                let bal = stream2.balance;
                game_with_data.game.is_finished = true;
                game_with_data.game.winner = Some(Player::First);
                self.games.insert(&game_id, &game_with_data);
                let game = game_with_data.game;
                if stream2.status == StreamStatus::Paused {
                    stop_stream(stream2.id.into())
                        .then(Promise::new(game.first_player.clone()).transfer(bal + bid.bid))
                } else {
                    Promise::new(game.first_player.clone()).transfer(bal + bid.bid)
                }
            }
            FinishedStreams::Second => {
                let bal = stream1.balance;
                game_with_data.game.is_finished = true;
                game_with_data.game.winner = Some(Player::First);
                self.games.insert(&game_id, &game_with_data);
                let game = game_with_data.game;
                if stream1.status == StreamStatus::Paused {
                    stop_stream(stream1.id.into())
                        .then(Promise::new(game.second_player.clone()).transfer(bal + bid.bid))
                } else {
                    Promise::new(game.second_player.clone()).transfer(bal + bid.bid)
                }
            }
            FinishedStreams::Both => {
                game_with_data.game.is_finished = true;
                self.games.insert(&game_id, &game_with_data);
                let game = game_with_data.game;
                Promise::new(game.first_player.clone())
                    .transfer(bid.bid)
                    .then(Promise::new(game.second_player.clone()).transfer(bid.bid))
            }
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
            let promise1 = if stream1.balance == stream1.available_to_withdraw_by_formula {
                stream1.status = StreamStatus::Finished {
                    reason: StreamFinishReason::StoppedByOwner,
                };
                stop_stream(stream1.id.into())
            } else {
                stream1.status = StreamStatus::Paused;
                pause_stream(stream1.id.into())
            };
            let promise2 = if stream2.balance == stream2.available_to_withdraw_by_formula {
                stream2.status = StreamStatus::Finished {
                    reason: StreamFinishReason::StoppedByOwner,
                };
                stop_stream(stream2.id.into())
            } else {
                stream2.status = StreamStatus::Paused;
                pause_stream(stream2.id.into())
            };
            (promise1.and(promise2))
                .then(Self::ext(env::current_account_id()).parse_two_streams(stream1, stream2))
        } else if stream1.status == StreamStatus::Active {
            let promise1 = if stream1.balance == stream1.available_to_withdraw_by_formula {
                stream1.status = StreamStatus::Finished {
                    reason: StreamFinishReason::StoppedByOwner,
                };
                stop_stream(stream1.id.into())
            } else {
                stream1.status = StreamStatus::Paused;
                pause_stream(stream1.id.into())
            };
            promise1.then(Self::ext(env::current_account_id()).parse_two_streams(stream1, stream2))
        } else if stream2.status == StreamStatus::Active {
            let promise2 = if stream2.balance == stream2.available_to_withdraw_by_formula {
                stream2.status = StreamStatus::Finished {
                    reason: StreamFinishReason::StoppedByOwner,
                };
                stop_stream(stream2.id.into())
            } else {
                stream2.status = StreamStatus::Paused;
                pause_stream(stream2.id.into())
            };
            promise2.then(Self::ext(env::current_account_id()).parse_two_streams(stream1, stream2))
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
        match (
            StatusType::new(stream1.status.clone()),
            StatusType::new(stream2.status.clone()),
        ) {
            (StatusType::Active, _) => unreachable!(),
            (_, StatusType::Active) => unreachable!(),
            (StatusType::Paused, StatusType::Paused) => (FinishedStreams::None, stream1, stream2),
            (StatusType::Finished, StatusType::Paused) => {
                (FinishedStreams::First, stream1, stream2)
            }
            (StatusType::Paused, StatusType::Finished) => {
                (FinishedStreams::Second, stream1, stream2)
            }
            (StatusType::Finished, StatusType::Finished) => {
                (FinishedStreams::Both, stream1, stream2)
            }
        }
    }
}
