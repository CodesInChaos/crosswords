use std::fmt::Debug;
use std::io::{stdin, BufRead};
use std::time::Instant;

const MINIMUM_WORD_LENGTH: usize = 3;

struct Problem {
    name: String,
    size: (usize, usize),
    accross: Vec<usize>,
    down: Vec<usize>,
}

impl Problem {
    fn load(input: &mut dyn BufRead) -> Problem {
        let mut header = String::new();
        let mut accross = String::new();
        let mut down = String::new();

        input.read_line(&mut header).unwrap();
        input.read_line(&mut accross).unwrap();
        input.read_line(&mut down).unwrap();

        let header = header.trim().split(": ").collect::<Vec<_>>();
        let accross = accross.trim().split(": ").collect::<Vec<_>>();
        let down = down.trim().split(": ").collect::<Vec<_>>();

        let name = header[0].to_owned();
        let size = header[1];
        let accross_label = accross[0];
        let accross = accross[1];
        let down_label = down[0];
        let down = down[1];

        assert_eq!("A", accross_label);
        assert_eq!("D", down_label);

        let size = size
            .split("x")
            .map(|s| s.parse().unwrap())
            .collect::<Vec<_>>();
        let size = (size[0], size[1]);
        let accross = accross.split(',').map(|s| s.parse().unwrap()).collect();
        let down = down.split(',').map(|s| s.parse().unwrap()).collect();

        Problem {
            name: name,
            size: size,
            accross: accross,
            down: down,
        }
    }

    fn in_bounds(&self, position: (isize, isize)) -> bool {
        position.0 >= 0
            && position.1 >= 0
            && position.0 < self.size.0 as isize
            && position.1 < self.size.1 as isize
    }

    fn field_count(&self) -> usize {
        self.size.0 * self.size.1
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Field {
    White,
    Black,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum FieldEx {
    Field(Field),
    OutOfBounds,
    Unfilled,
}

#[derive(Clone)]
struct Counts {
    number: usize,
    accross_used: usize,
    down_used: usize,
    reverse_number: usize,
    reverse_accross_used: usize,
    reverse_down_used: usize,
}

struct State<'p> {
    problem: &'p Problem,
    fields: Vec<Field>,
    counts: Vec<Counts>,
}

struct Scan<'s> {
    state: &'s State<'s>,
    position: (isize, isize),
    direction: (isize, isize),
}

impl<'s> Iterator for Scan<'s> {
    type Item = FieldEx;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.state.try_at(self.position);
        self.position.0 += self.direction.0;
        self.position.1 += self.direction.1;
        Some(result)
    }
}

#[derive(Debug)]
enum RuleViolation {
    NumberWrongAccross,
    NumberWrongDown,
    NumberWrongAccrossReverse,
    NumberWrongDownReverse,
    WordTooShortAccross,
    WordTooShortDown,
    TooLittleSpaceAccross,
    TooLittleSpaceDown,
    LeftOverAccross,
    LeftOverDown,
}

impl<'p> State<'p> {
    fn is_final(&self) -> bool {
        self.fields.len() == self.problem.size.0 * self.problem.size.1
    }

    fn try_at(&self, position: (isize, isize)) -> FieldEx {
        if !self.problem.in_bounds(position) {
            return FieldEx::OutOfBounds;
        }
        let field_count = self.problem.field_count();
        let mut index = (position.1 * self.problem.size.0 as isize + position.0) as usize;
        if index > field_count / 2 {
            index = field_count - 1 - index;
        }
        self.fields
            .get(index)
            .map_or(FieldEx::Unfilled, |f| FieldEx::Field(*f))
    }

    fn at(&self, position: (isize, isize)) -> Field {
        match self.try_at(position) {
            FieldEx::Field(f) => f,
            FieldEx::OutOfBounds => Field::Black,
            FieldEx::Unfilled => unreachable!("unfilled"),
        }
    }

    fn scan(&self, start: (isize, isize), dir: (isize, isize)) -> Scan {
        Scan {
            state: self,
            position: start,
            direction: dir,
        }
    }

    fn new(problem: &'p Problem) -> Self {
        State {
            problem: problem,
            fields: Vec::<_>::new(),
            counts: vec![Counts {
                number: 1,
                accross_used: 0,
                down_used: 0,
                reverse_number: problem
                    .accross
                    .iter()
                    .chain(problem.down.iter())
                    .copied()
                    .max()
                    .unwrap(),
                reverse_down_used: 0,
                reverse_accross_used: 0,
            }],
        }
    }

    fn push(&mut self, field: Field) -> Result<(), RuleViolation> {
        let remaining = (self.problem.size.0 * self.problem.size.1) as isize
            - (self.fields.len() * 2 + 1) as isize;
        assert!(remaining >= 0);
        self.push_one(field)?;
        if remaining == 0 {
            for i in (0..self.fields.len() - 1).rev() {
                let field = self.fields[i];
                self.push_one(field)?;
            }
        }
        Ok(())
    }

    fn push_one(&mut self, field: Field) -> Result<(), RuleViolation> {
        let problem = self.problem;
        let position = (
            (self.fields.len() % problem.size.0) as isize,
            (self.fields.len() / problem.size.0) as isize,
        );
        assert!(problem.in_bounds(position));
        if position.1 * 2 >= problem.size.1 as isize {
            let expected = self.at((
                problem.size.0 as isize - 1 - position.0,
                problem.size.1 as isize - 1 - position.1,
            ));
            assert!(field == expected);
        }
        self.fields.push(field);
        let mut counts = self.counts.last().unwrap().clone();

        match field {
            Field::White => {
                let mut numbered = false;
                if self.at((position.0 - 1, position.1)) == Field::Black {
                    // starts a new "accross" word

                    // check for correct number
                    numbered = true;
                    if self.problem.accross.get(counts.accross_used).copied() != Some(counts.number)
                    {
                        return Err(RuleViolation::NumberWrongAccross);
                    }

                    // newly started word has enough space to the right to fit the minimum word length
                    counts.accross_used += 1;
                    if problem.size.0 as isize - position.0 < MINIMUM_WORD_LENGTH as isize {
                        return Err(RuleViolation::TooLittleSpaceAccross);
                    }
                }
                if self.at((position.0, position.1 - 1)) == Field::Black {
                    // starts a new "down" word

                    // check for correct number
                    numbered = true;
                    if self.problem.down.get(counts.down_used).copied() != Some(counts.number) {
                        return Err(RuleViolation::NumberWrongDown);
                    }

                    // newly started word has enough space below to fit the minimum word length
                    counts.down_used += 1;
                    let down_white = self
                        .scan(position, (0, 1))
                        .take(MINIMUM_WORD_LENGTH)
                        .take_while(|f| {
                            *f == FieldEx::Field(Field::White) || *f == FieldEx::Unfilled
                        })
                        .count();
                    if down_white < MINIMUM_WORD_LENGTH {
                        return Err(RuleViolation::TooLittleSpaceDown);
                    }
                }
                if numbered {
                    counts.number += 1;
                }
            }
            Field::Black => {
                // a back field can end a word
                // check if word on the left satisfies minimum word length
                let left_white = self
                    .scan(position, (-1, 0))
                    .skip(1)
                    .take(MINIMUM_WORD_LENGTH)
                    .take_while(|f| *f == FieldEx::Field(Field::White))
                    .count();
                if left_white != 0 && left_white < MINIMUM_WORD_LENGTH {
                    return Err(RuleViolation::WordTooShortAccross);
                }

                // check if word above satisfies minimum word length
                let up_white = self
                    .scan(position, (0, -1))
                    .skip(1)
                    .take(MINIMUM_WORD_LENGTH)
                    .take_while(|f| *f == FieldEx::Field(Field::White))
                    .count();
                if up_white != 0 && up_white < MINIMUM_WORD_LENGTH {
                    return Err(RuleViolation::WordTooShortDown);
                }
            }
        }

        // check the mirrored position. We have to go one row further down, since the numbers are only determined once the row above is filled
        // relative positions are negated, since we're looking at the upper left version of the board, when we actually check the rules for the lower right version of the board
        if self.at((position.0, position.1 - 1)) == Field::White {
            let position = (position.0, position.1 - 1);

            let mut numbered = false;
            if self.at((position.0, position.1 + 1)) == Field::Black {
                // starts a new "accross" word (reversed)

                // check for correct number
                numbered = true;
                if self
                    .problem
                    .down
                    .get(problem.down.len() - 1 - counts.reverse_down_used)
                    .copied()
                    != Some(counts.reverse_number)
                {
                    return Err(RuleViolation::NumberWrongDownReverse);
                }
                counts.reverse_down_used += 1;
            }
            if self.at((position.0 + 1, position.1)) == Field::Black {
                // starts a new "down" word (reversed)

                // check for correct number
                numbered = true;
                if self
                    .problem
                    .accross
                    .get(problem.accross.len() - 1 - counts.reverse_accross_used)
                    .copied()
                    != Some(counts.reverse_number)
                {
                    return Err(RuleViolation::NumberWrongAccrossReverse);
                }
                counts.reverse_accross_used += 1;
            }
            if numbered {
                counts.reverse_number -= 1;
            }
        }

        // once the board is filled, we must have used up all numbers
        if self.fields.len() == problem.field_count() {
            if counts.accross_used != problem.accross.len() {
                return Err(RuleViolation::LeftOverAccross);
            }

            if counts.down_used != problem.down.len() {
                return Err(RuleViolation::LeftOverDown);
            }
        }

        self.counts.push(counts);
        Ok(())
    }
}

impl<'p> Debug for State<'p> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for y in 0..self.problem.size.1 {
            for x in 0..self.problem.size.0 {
                let field = self.try_at((x as isize, y as isize));
                match field {
                    FieldEx::Field(field) => match field {
                        Field::White => f.write_str(". ")?,
                        Field::Black => f.write_str("# ")?,
                    },
                    FieldEx::OutOfBounds => unreachable!("out of bounds"),
                    FieldEx::Unfilled => f.write_str("? ")?,
                }
            }
            f.write_str("\r\n")?;
        }
        f.write_str("\r\n")?;
        Ok(())
    }
}

struct SearchState {
    start: Instant,
    solution_count: u64,
    error_count: u64,
}

fn search<'p>(state: &mut State<'p>, search_state: &mut SearchState) {
    let len = state.fields.len();
    for field in [Field::White, Field::Black].iter() {
        match state.push(*field) {
            Ok(_) => {
                if state.is_final() {
                    search_state.solution_count += 1;
                    println!("{:?}", state);
                } else {
                    search(state, search_state);
                }
            }
            Err(error) => {
                search_state.error_count += 1;
                if search_state.error_count % 10_000_000 == 0 {
                    eprintln!(
                        "Solutions: {}, Elapsed: {}s, Errors: {}M, {:?}",
                        search_state.solution_count,
                        search_state.start.elapsed().as_secs(),
                        search_state.error_count / 1_000_000,
                        error
                    );
                    eprintln!("{:?}", state);
                }
            }
        }

        state.fields.truncate(len);
        state.counts.truncate(len + 1);
    }
}

fn search_problem<'p>(problem: &'p Problem) -> SearchState {
    let mut search_state = SearchState {
        start: Instant::now(),
        solution_count: 0,
        error_count: 0,
    };
    let mut state = State::new(problem);
    search(&mut state, &mut search_state);
    search_state
}

fn check_example_solution() {
    let problem_text =
        "EXAMPLE: 15x15
        A: 1,4,7,10,13,14,16,17,18,20,21,23,24,26,28,29,33,35,36,38,39,42,44,45,47,49,50,52,55,56,58,59,61,63,67,69,70,71,72,73,74,75,76
        D: 1,2,3,4,5,6,7,8,9,11,12,15,19,22,25,27,29,30,31,32,34,37,40,41,43,46,48,51,53,54,57,60,62,64,65,66,68";
    let solution="...###...#...##.....#...#....#.....#...#..........#....#....###...#....#..........#..........##...#...........#...#...........#...##..........#..........#....#...###....#....#..........#...#.....#....#...#.....##...#...###...";

    let problem = Problem::load(&mut problem_text.as_bytes());
    let mut state = State::new(&problem);

    for c in solution.chars() {
        let field;
        match c {
            '#' => field = Field::Black,
            '.' => field = Field::White,
            _ => unreachable!(),
        }
        if let Err(error) = state.push_one(field) {
            println!("{:?}", error);
            println!("{:?}", state);
            break;
        }
    }
}

fn main() {
    check_example_solution();

    let problem = Problem::load(&mut stdin().lock());
    println!(
        "Problem: {} ({}x{})",
        problem.name, problem.size.0, problem.size.1
    );
    println!();

    let result = search_problem(&problem);
    println!("solutions: {}", result.solution_count);
    println!("errors: {}", result.error_count);
    println!("duration: {:?}", result.start.elapsed());
}
