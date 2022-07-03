use acpc_server_sys as acpc;
use libc;

use std::fs::File;
use std::os::unix::io::IntoRawFd;
use std::ffi::CStr;
use std::ffi::CString;
use std::fmt;


pub type Card = u8;


/// Available actions in a game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Action {
    /// Fold action.
    Fold,

    /// Call action.
    Call,

    /// Raise action with a specified amount.
    Raise(i32),

    /// Invalid
    Invalid,
}

fn to_acpc_action(action: &Action) -> acpc::Action {
    match action {
	Action::Fold => acpc::Action{type_: acpc::ActionType_a_fold, size: 0},
	Action::Call => acpc::Action{type_: acpc::ActionType_a_call, size: 0},
	Action::Raise(size) => acpc::Action{type_: acpc::ActionType_a_raise, size: *size},
	Action::Invalid => acpc::Action{type_: acpc::ActionType_a_invalid, size: 0},
    }
}


#[derive(Debug, Clone)]
pub struct Game {
    hand_id: u32,
    game_: acpc::Game,
}

impl Game {
    pub fn read(file: File) -> Self {
	let hand_id = 0;
	let game_ = unsafe {
	    let c_file = libc::fdopen(
		file.into_raw_fd(),
		CStr::from_bytes_with_nul_unchecked(b"r\0").as_ptr(),
            );
	    let game = acpc::readGame(c_file as *mut acpc::_IO_FILE);
	    libc::fclose(c_file);
	    *game
	};
	return Game { hand_id, game_ };
    }

    pub fn print(&self, file: File) {
	let game_ptr = &self.game_ as *const acpc::Game;
	unsafe {
	    let c_file = libc::fdopen(
		file.into_raw_fd(),
		CStr::from_bytes_with_nul_unchecked(b"r\0").as_ptr(),
            );
	    acpc::printGame(c_file as *mut acpc::_IO_FILE, game_ptr);
	}
    }

    pub fn number_of_players(&self) -> u8 {
	self.game_.numPlayers
    }

    pub fn bc_start(&self, round: u8) -> u8 {
	let game_ptr = &self.game_ as *const acpc::Game;
	unsafe {
	    acpc::bcStart(game_ptr, round)
	}
    }
    
    pub fn sum_board_cards(&self, round: u8) -> u8 {
	let game_ptr = &self.game_ as *const acpc::Game;
	unsafe {
	    acpc::sumBoardCards(game_ptr, round)
	}
    }

    fn player_idx(&self, player: u8) -> Result<usize, String> {
	if self.number_of_players() <= player {
	    Err(format!("Invalid player index {}", player))
	} else {
	    Ok(player as usize)
	}
    }

    fn num_hole_cards(&self) -> u8 {
	self.game_.numHoleCards
    }

    pub fn stack_size(&self, player: u8) -> Result<i32, String> {
	Ok(self.game_.stack[self.player_idx(player)?])
    }

    pub fn blind_size(&self, player: u8) -> Result<i32, String> {
	Ok(self.game_.blind[self.player_idx(player)?])
    }

    pub fn total_money(&self) -> i64 {
	let n = self.number_of_players() as usize;
	self.game_.stack.iter().take(n).fold(0, |sum, i| sum + (*i as i64))
    }
}


#[derive(Debug, Clone)]
pub struct State{
    game: Game,
    state_: acpc::State,
}

impl State {
    fn new_acpc_state() -> acpc::State {
	acpc::State {
	    handId: 0u32,
	    maxSpent: 0i32,
	    minNoLimitRaiseTo: 0i32,
	    spent: [0i32; 10usize],
	    action: [[acpc::Action{ type_: 0u32, size: 0i32 }; 64usize]; 4usize],
	    actingPlayer: [[0u8; 64usize]; 4usize],
	    numActions: [0u8; 4usize],
	    round: 0u8,
	    finished: 0u8,
	    playerFolded: [0u8; 10usize],
	    boardCards: [0u8; 7usize],
	    holeCards: [[0u8; 3usize]; 10usize],
	}
    }
    
    pub fn new(game: Game) -> Self {
	let mut state_ = State::new_acpc_state();
	let state_ptr = &mut state_ as *mut acpc::State;
	let game_ptr = &game.game_ as *const acpc::Game;
	unsafe {
	    acpc::initState(game_ptr, game.hand_id, state_ptr);
	};
	return State{ game, state_ };
    }

    #[inline]
    pub fn spent(&self, player: u8) -> Result<i32, String> {
	Ok(self.state_.spent[self.game.player_idx(player)?])
    }

    #[inline]
    pub fn max_spend(&self) -> u32 {
	self.state_.maxSpent as u32
    }

    #[inline]
    pub fn value_of_state(&self, player: u8) -> Result<f64, String> {
	if !self.is_finished() {
	    return Err("Game is not finished".to_owned());
	}
	let state_ptr = &self.state_ as *const acpc::State;
	let game_ptr = &self.game.game_ as *const acpc::Game;
	let player_idx = self.game.player_idx(player)?;
	unsafe {
	    Ok(acpc::valueOfState(game_ptr, state_ptr, player_idx as u8))
	}
    }

    #[inline]
    pub fn raise_size(&self) -> Result<(i32, i32), String> {
	let mut min_size = 0;
	let mut max_size = 0;
	let state_ptr = &self.state_ as *const acpc::State;
	let game_ptr = &self.game.game_ as *const acpc::Game;
	let min_size_ptr = &mut min_size as *mut i32;
	let max_size_ptr = &mut max_size as *mut i32;
	let result = unsafe {
	    acpc::raiseIsValid(game_ptr, state_ptr, min_size_ptr, max_size_ptr)
	};
	match result {
	    0 => Err("player Can not raise now.".to_owned()),
	    1 => Ok((min_size, max_size)),
	    _ => panic!("Invalid result from acpc::isValidAction {}", result),
	}
    }

    #[inline]
    pub fn num_folded(&self) -> u8 {
	let state_ptr = &self.state_ as *const acpc::State;
	let game_ptr = &self.game.game_ as *const acpc::Game;
	unsafe {
	    acpc::numFolded(game_ptr, state_ptr)
	}
    }

    #[inline]
    pub fn current_player(&self) -> u8 {
	let state_ptr = &self.state_ as *const acpc::State;
	let game_ptr = &self.game.game_ as *const acpc::Game;
	unsafe {
	    acpc::currentPlayer(game_ptr, state_ptr)
	}
    }

    pub fn do_action(&mut self, action: Action) -> Result<(), &str>{
	if !self.is_valid_action(action) {
	    return Err("Invalid Action");
	}
	let acpc_action = to_acpc_action(&action);
	let state_ptr = &mut self.state_ as *mut acpc::State;
	let game_ptr = &self.game.game_ as *const acpc::Game;
	let action_ptr = &acpc_action as *const acpc::Action;
	unsafe {
	    acpc::doAction(game_ptr, action_ptr, state_ptr)
	}
	Ok(())
    }

    #[inline]
    pub fn is_valid_action(&self, action: Action) -> bool {
	let mut acpc_action = to_acpc_action(&action);
	let auto_fix_action_in_c = 0;
	let state_ptr = &self.state_ as *const acpc::State;
	let game_ptr = &self.game.game_ as *const acpc::Game;
	let action_ptr = &mut acpc_action as *mut acpc::Action;
	let result = unsafe {
	    acpc::isValidAction(game_ptr, state_ptr, auto_fix_action_in_c, action_ptr)
	};
	match result {
	    0 => false, // invalid Action
	    1 => true, // valid action
	    _ => panic!("Invalid result from acpc::isValidAction {}", result),
	}
    }

    #[inline]
    pub fn is_finished(&self) -> bool {
	match self.state_.finished {
	    0 => false,
	    1 => true,
	    _ => panic!("Invalid bool in u8 {}", self.state_.finished),
	}	
    }

    #[inline]
    pub fn money(&self, player: u8) -> Result<i32, String> {
	Ok(self.game.stack_size(player)? - self.spent(player)?)
    }

    #[inline]
    pub fn ante(&self, player: u8) -> Result<i32, String> {
	Ok(self.spent(player)?)
    }
    
    #[inline]
    pub fn total_spent(&self) -> i32 {
	let n = self.game.number_of_players() as usize;
	self.state_.spent.iter().take(n).fold(0, |sum, i| sum + *i)
    }

    #[inline]
    pub fn current_spent(&self, player: u8) -> Result<i32, String> {
	Ok(self.spent(player)?)
    }

    pub fn set_hole_cards(&mut self, player: u8, cards: &[Card]) -> Result<(), String> {
	assert!(self.game.num_hole_cards() as usize == cards.len());
	let mut fixed_size_cards: [Card; 3] = [0; 3];
	for (i, v) in cards.into_iter().enumerate() {
	    fixed_size_cards[i] = *v;
	}
	self.state_.holeCards[self.game.player_idx(player)?] = fixed_size_cards;
	Ok(())
    }

    #[inline]
    pub fn hole_cards(&self, player: u8) -> Result<&[Card], String> {
	let length = self.game.game_.numHoleCards as usize;
	Ok(&self.state_.holeCards[self.game.player_idx(player)?][..length])
    }

    pub fn set_board_cards(&mut self, cards: &[Card]) -> Result<(), String> {
	assert!(self.game.sum_board_cards(self.get_round()) as usize == cards.len());
	let mut fixed_size_cards: [Card; 7] = [0; 7];
	for (i, v) in cards.into_iter().enumerate() {
	    fixed_size_cards[i] = *v;
	}
	self.state_.boardCards = fixed_size_cards;
	Ok(())
    }

    #[inline]
    pub fn board_cards(&self) -> Result<&[Card], String> {
	let length = self.game.sum_board_cards(self.get_round()) as usize;
	Ok(&self.state_.boardCards[..length])
    }    

    #[inline]
    pub fn get_round(&self) -> u8 {
	self.state_.round
    }

    pub fn deal_cards(&self) {
	//
    }
}


impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
	// https://docs.rs/rustc-std-workspace-std/1.0.1/std/ffi/struct.CString.html#examples-3
	let string = CString::new(" ".repeat(4096)).expect("CString::new failed");
	let ptr = string.into_raw();
	let state_ptr = &self.state_ as *const acpc::State;
	let game_ptr = &self.game.game_ as *const acpc::Game;
	let c_string = unsafe {
	    let _ = acpc::printState(game_ptr, state_ptr, 4096, ptr);
	    CString::from_raw(ptr)
	};
        write!(f, "{:?}", &c_string)
    }
}


#[cfg(test)]
mod game_tests {
    use super::*;
    use std::fs::File;

    fn get_game() -> Game {
	let file = File::open("resources/leduc.limit.2p.game").unwrap();
	Game::read(file)
    }

    fn get_game_nolimit() -> Game {
	let file = File::open("resources/holdem.nolimit.3p.game").unwrap();
	Game::read(file)
    }

    #[test]
    fn bc_start() {
	let game = get_game();
	assert_eq!(0, game.bc_start(0));
	assert_eq!(0, game.bc_start(1));
    }    
    
    #[test]
    fn sum_board_cards() {
	let game = get_game();
	assert_eq!(0, game.sum_board_cards(0));
	assert_eq!(1, game.sum_board_cards(1));
    }

    #[test]
    fn stack_size() {
	let game = get_game();
	assert_eq!(game.stack_size(0), Ok(i32::MAX));
	assert_eq!(game.stack_size(1), Ok(i32::MAX));
	assert!(game.stack_size(2).is_err());

	let game = get_game_nolimit();
	assert_eq!(game.stack_size(0), Ok(20000));
	assert_eq!(game.stack_size(1), Ok(20000));
	assert_eq!(game.stack_size(2), Ok(20000));
	assert!(game.stack_size(3).is_err());
    }
    
    #[test]
    fn blind_size() {
	let game = get_game();
	assert_eq!(game.blind_size(0), Ok(1));
	assert_eq!(game.blind_size(1), Ok(1));
	assert!(game.stack_size(2).is_err());
    }

    #[test]
    fn total_money() {
	let game = get_game();
	assert_eq!(game.total_money(), (i32::MAX as i64) * 2);
    }
}

#[cfg(test)]
mod state_tests {
    use super::*;
    use std::fs::File;

    fn get_state() -> State {
	let file = File::open("resources/holdem.nolimit.3p.game").unwrap();
	let game = Game::read(file);
	State::new(game)
    }

    fn play_until_showdown(state: &mut State) {
	loop {
	    if state.is_finished() {
		return 
	    }
	    let action = Action::Call;
	    state.do_action(action).unwrap();
	}
    }

    #[test]
    fn new() {
	println!("{}", get_state());
    }

    #[test]
    fn value_of_state() {
	let mut state = get_state();
	assert!(state.value_of_state(0).is_err());
	play_until_showdown(&mut state);
	assert_eq!(Ok(0.0), state.value_of_state(0));
	assert_eq!(Ok(0.0), state.value_of_state(1));
	assert_eq!(Ok(0.0), state.value_of_state(2));
	assert!(state.value_of_state(3).is_err());
    }

    #[test]
    fn raise_size() {
	let mut state = get_state();
	assert_eq!(Ok(50), state.current_spent(0));
	assert_eq!(Ok(100), state.current_spent(1));
	assert_eq!(Ok(0), state.current_spent(2));

	assert_eq!(Ok((200, 20000)), state.raise_size());
	state.do_action(Action::Raise(200)).unwrap();
	assert_eq!(Ok(50), state.current_spent(0));
	assert_eq!(Ok(100), state.current_spent(1));
	assert_eq!(Ok(200), state.current_spent(2));

	assert_eq!(Ok((300, 20000)), state.raise_size());
	state.do_action(Action::Raise(1000)).unwrap();
	assert_eq!(Ok(1000), state.current_spent(0));
	assert_eq!(Ok(100), state.current_spent(1));
	assert_eq!(Ok(200), state.current_spent(2));
	
	assert_eq!(Ok((1800, 20000)), state.raise_size());
	state.do_action(Action::Raise(20000)).unwrap();
	assert_eq!(Ok(1000), state.current_spent(0));
	assert_eq!(Ok(20000), state.current_spent(1));
	assert_eq!(Ok(200), state.current_spent(2));
    }

    #[test]
    fn num_folded() {
	let mut state = get_state();
	assert_eq!(0, state.num_folded());
	state.do_action(Action::Fold).unwrap();
	assert_eq!(1, state.num_folded());
    }

    #[test]
    fn current_player() {
	let mut state = get_state();
	assert_eq!(2, state.current_player());
	state.do_action(Action::Fold).unwrap();
	assert_eq!(0, state.current_player());
	state.do_action(Action::Raise(200)).unwrap();
	assert_eq!(1, state.current_player());
	state.do_action(Action::Raise(500)).unwrap();
	assert_eq!(0, state.current_player());
    }

    #[test]
    fn is_valid_action() {
	let a_fold = Action::Fold;
	let a_call = Action::Call;
	let a_raise_100 = Action::Raise(100);
	let a_raise_1000 = Action::Raise(1000);
	let a_raise_10000 = Action::Raise(10000);
	let a_raise_20001 = Action::Raise(20001);

	let mut state = get_state();
	assert_eq!(true, state.is_valid_action(Action::Fold));
	assert_eq!(true, state.is_valid_action(Action::Call));
	assert_eq!(false, state.is_valid_action(Action::Raise(100)));
	assert_eq!(true, state.is_valid_action(Action::Raise(1000)));
	assert_eq!(true, state.is_valid_action(Action::Raise(10000)));
	assert_eq!(false, state.is_valid_action(Action::Raise(20001)));
	
	state.do_action(Action::Raise(1000)).unwrap();
	assert_eq!(true, state.is_valid_action(Action::Fold));
	assert_eq!(true, state.is_valid_action(Action::Call));
	assert_eq!(false, state.is_valid_action(Action::Raise(100)));
	assert_eq!(false, state.is_valid_action(Action::Raise(1000)));
	assert_eq!(true, state.is_valid_action(Action::Raise(10000)));
	assert_eq!(false, state.is_valid_action(Action::Raise(20001)));
    }

    #[test]
    fn money_and_ante() {
	let mut state = get_state();
	state.do_action(Action::Raise(200)).unwrap(); // 2
	state.do_action(Action::Raise(1000)).unwrap(); // 0
	state.do_action(Action::Call).unwrap(); // 1
	state.do_action(Action::Call).unwrap(); // 2
	state.do_action(Action::Raise(2000)).unwrap(); // 0
	assert_eq!(Ok(18000), state.money(0));
	assert_eq!(Ok(19000), state.money(1));
	assert_eq!(Ok(19000), state.money(2));

	assert_eq!(Ok(2000), state.ante(0));
	assert_eq!(Ok(1000), state.ante(1));
	assert_eq!(Ok(1000), state.ante(2));

	assert_eq!(4000, state.total_spent());
    }

    #[test]
    fn showdown() {
	let hole_cards = [[1, 35], [5, 50], [11, 51]];
	let board = [17, 19, 23, 29, 37];
	let mut state = get_state();
	play_until_showdown(&mut state);
	for (i, cards) in hole_cards.iter().enumerate() {
	    state.set_hole_cards(i as u8, cards).unwrap();
	}
	for (i, cards) in hole_cards.iter().enumerate() {
	    assert_eq!(Ok(&cards[..]), state.hole_cards(i as u8));
	}
	state.set_board_cards(&board).unwrap();
	assert_eq!(Ok(&board[..]), state.board_cards());
	// println!("{}", state);
	assert_eq!(Ok(-100.0), state.value_of_state(0)); // lose
	assert_eq!(Ok(50.0), state.value_of_state(1)); // tie
	assert_eq!(Ok(50.0), state.value_of_state(2)); // tie
    }
    
}
