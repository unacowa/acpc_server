use std::fs::File;

use acpc_server::Game;
use acpc_server::State;
use acpc_server::Action;


#[derive(Debug, Clone)]
struct Node<'a> {
    parent: Option<&'a Node<'a>>,
    player: u8,
    action: Action,
}

impl<'a> Node<'a> {
    fn history(&self) -> Vec<String> {
	let mut his: Vec<String> = vec![];
	let mut node = self;
	loop {
	    his.push(format!("P{}|{:?}", node.player, node.action));
	    match node.parent {
		Some(a) => node = a,
		None => break,
	    };
	};
	his.into_iter().rev().collect::<Vec<_>>()
    }
}

fn legal_actions(state: &State) -> Vec<Action> {
    let actions = match state.get_round() {
	0 => vec![Action::Fold,
		  Action::Call,
		  Action::Raise(2)],
	1 => vec![Action::Fold,
		  Action::Call,
		  Action::Raise(4)],
	_ => panic!("Invalid round {}", state.get_round())
    };
    actions.into_iter().filter(|a| state.is_valid_action(*a)).collect::<Vec<_>>()
}

fn traverse(state: State, parent: Option<&Node>) {
    if state.is_finished() {
	println!("{:?} [{:?} {:?}]",
		 parent.unwrap().history(),
		 state.value_of_state(0),
		 state.value_of_state(1),
	);
    }
    // let mut line = String::new();
    // std::io::stdin().read_line(&mut line).expect("Failed to read line");
    for action in legal_actions(&state) {
	// match parent {
	//     Some(node) => println!("{:?} -> {:?}", node.history(), action),
	//     None => println!("ROOT -> {:?}", action),
	// };
	let node = Node { parent, player: state.current_player(),
			  action: action.clone() };
	let mut next_state = state.clone();
	next_state.do_action(action).unwrap();
	traverse(next_state, Some(&node));
    }
}


fn main() {
    let file = File::open("resources/leduc.limit.2p.game").unwrap();
    let game = Game::read(file);
    let state = State::new(game);

    traverse(state, None);
}
