/// The two-dimensional facing subset of BYOND's direction type.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum Dir {
    North = 1,
    #[default]
    South = 2,
    East = 4,
    West = 8,
    Northeast = 5,
    Northwest = 9,
    Southeast = 6,
    Southwest = 10,
}

impl Dir {
    pub const CARDINALS: &'static [Dir] = &[Dir::North, Dir::South, Dir::East, Dir::West];
    pub const DIAGONALS: &'static [Dir] = &[
        Dir::Northeast,
        Dir::Northwest,
        Dir::Southeast,
        Dir::Southwest,
    ];
    pub const ALL: &'static [Dir] = &[
        Dir::North,
        Dir::South,
        Dir::East,
        Dir::West,
        Dir::Northeast,
        Dir::Northwest,
        Dir::Southeast,
        Dir::Southwest,
    ];

    /// Attempt to build a direction from its integer representation.
    pub fn from_int(int: i32) -> Option<Dir> {
        Some(match int {
            1 => Dir::North,
            2 => Dir::South,
            4 => Dir::East,
            8 => Dir::West,
            5 => Dir::Northeast,
            9 => Dir::Northwest,
            6 => Dir::Southeast,
            10 => Dir::Southwest,
            _ => return None,
        })
    }

    /// Get this direction's integer representation.
    pub fn to_int(self) -> i32 {
        self as i32
    }

    pub fn contains(self, other: Dir) -> bool {
        self.to_int() & other.to_int() != 0
    }

    pub fn is_diagonal(self) -> bool {
        !matches!(self, Dir::North | Dir::South | Dir::East | Dir::West)
    }

    pub fn flip(self) -> Dir {
        match self {
            Dir::North => Dir::South,
            Dir::South => Dir::North,
            Dir::East => Dir::West,
            Dir::West => Dir::East,
            Dir::Northeast => Dir::Southwest,
            Dir::Northwest => Dir::Southeast,
            Dir::Southeast => Dir::Northwest,
            Dir::Southwest => Dir::Northeast,
        }
    }

    pub fn flip_ns(self) -> Dir {
        match self {
            Dir::North => Dir::South,
            Dir::South => Dir::North,
            Dir::East => Dir::East,
            Dir::West => Dir::West,
            Dir::Northeast => Dir::Southeast,
            Dir::Northwest => Dir::Southwest,
            Dir::Southeast => Dir::Northeast,
            Dir::Southwest => Dir::Northwest,
        }
    }

    pub fn flip_ew(self) -> Dir {
        match self {
            Dir::North => Dir::North,
            Dir::South => Dir::South,
            Dir::East => Dir::West,
            Dir::West => Dir::East,
            Dir::Northeast => Dir::Northwest,
            Dir::Northwest => Dir::Northeast,
            Dir::Southeast => Dir::Southwest,
            Dir::Southwest => Dir::Southeast,
        }
    }

    pub fn clockwise_45(self) -> Dir {
        match self {
            Dir::North => Dir::Northeast,
            Dir::Northeast => Dir::East,
            Dir::East => Dir::Southeast,
            Dir::Southeast => Dir::South,
            Dir::South => Dir::Southwest,
            Dir::Southwest => Dir::West,
            Dir::West => Dir::Northwest,
            Dir::Northwest => Dir::North,
        }
    }

    pub fn counterclockwise_45(self) -> Dir {
        match self {
            Dir::North => Dir::Northwest,
            Dir::Northeast => Dir::North,
            Dir::East => Dir::Northeast,
            Dir::Southeast => Dir::East,
            Dir::South => Dir::Southeast,
            Dir::Southwest => Dir::South,
            Dir::West => Dir::Southwest,
            Dir::Northwest => Dir::West,
        }
    }

    pub fn clockwise_90(self) -> Dir {
        match self {
            Dir::North => Dir::East,
            Dir::South => Dir::West,
            Dir::East => Dir::South,
            Dir::West => Dir::North,
            Dir::Northeast => Dir::Southeast,
            Dir::Northwest => Dir::Northeast,
            Dir::Southeast => Dir::Southwest,
            Dir::Southwest => Dir::Northeast,
        }
    }

    pub fn counterclockwise_90(self) -> Dir {
        match self {
            Dir::North => Dir::West,
            Dir::South => Dir::East,
            Dir::East => Dir::North,
            Dir::West => Dir::South,
            Dir::Southeast => Dir::Northeast,
            Dir::Northeast => Dir::Northwest,
            Dir::Southwest => Dir::Southeast,
            Dir::Northwest => Dir::Southwest,
        }
    }

    /// Get this direction's offset in BYOND's coordinate system.
    pub fn offset(self) -> (i32, i32) {
        match self {
            Dir::North => (0, 1),
            Dir::South => (0, -1),
            Dir::East => (1, 0),
            Dir::West => (-1, 0),
            Dir::Northeast => (1, 1),
            Dir::Northwest => (-1, 1),
            Dir::Southeast => (1, -1),
            Dir::Southwest => (-1, -1),
        }
    }
}
