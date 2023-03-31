use crate::consts::{BOARD_X, BOARD_X_Y, BOARD_Y, BOARD_Y_VALID, BRICKS_TYPES, BRICK_VIEWS};
use bevy::prelude::Resource;
use rand::prelude::*;
#[derive(Copy, Clone, Default, Debug)]
pub struct Dot(pub i8, pub i8);

impl Dot {
    pub fn with_original_dot(&self, pos: &Dot) -> Self {
        Self(self.0 + pos.0, self.1 + pos.1)
    }
    pub fn move_left(&mut self) {
        self.0 -= 1;
    }
    pub fn move_right(&mut self) {
        self.0 += 1;
    }
    pub fn move_down(&mut self) {
        self.1 -= 1;
    }
    pub fn left(&self) -> Self {
        Self(self.0 - 1, self.1)
    }
    pub fn right(&self) -> Self {
        Self(self.0 + 1, self.1)
    }
    pub fn down(&self) -> Self {
        Self(self.0, self.1 - 1)
    }
}

#[derive(Copy, Clone)]
pub struct BrickView {
    pub dots: [Dot; 4],
}

#[derive(Copy, Clone, Resource)]
pub struct Brick {
    pub ty: usize,
    pub rotation: usize,
}

impl From<Brick> for BrickView {
    fn from(bs: Brick) -> BrickView {
        BRICK_VIEWS[bs.ty][bs.rotation]
    }
}

impl Brick {
    pub fn rand() -> Self {
        let ty = rand::thread_rng().gen_range(0..BRICKS_TYPES);
        Self { ty, rotation: 0 }
    }
    pub fn rotate(&self) -> Self {
        Self {
            ty: self.ty,
            rotation: (self.rotation + 1) % BRICK_VIEWS[self.ty].len(),
        }
    }
}

#[derive(Debug)]
pub struct Board(Vec<bool>);

impl Default for Board {
    fn default() -> Self {
        Self(vec![false; BOARD_X_Y])
    }
}
impl Board {
    fn index(dot: &Dot) -> usize {
        dot.0 as usize + dot.1 as usize * BOARD_X as usize
    }
    pub fn occupy_dot(&mut self, dot: &Dot) -> &mut Self {
        let i = Self::index(dot);
        if i < BOARD_X_Y {
            self.0[i] = true
        }
        self
    }
    pub fn occupy_brick_view(&mut self, brick: &BrickView, pos: &Dot) {
        for i in 0..4 {
            self.occupy_dot(&brick.dots[i].with_original_dot(pos));
        }
    }

    pub fn occupy_brick(&mut self, brick: &Brick, pos: &Dot) {
        let brick = BrickView::from(*brick);
        self.occupy_brick_view(&brick, pos)
    }

    pub fn occupied_dot(&self, dot: &Dot) -> bool {
        let i = Self::index(dot);
        if i < BOARD_X_Y {
            self.0[i]
        } else {
            false
        }
    }
    pub fn conflict_brick(&self, brick: &BrickView, pos: &Dot) -> bool {
        self.occupied_dot(&brick.dots[0].with_original_dot(pos))
            || self.occupied_dot(&brick.dots[1].with_original_dot(pos))
            || self.occupied_dot(&brick.dots[2].with_original_dot(pos))
            || self.occupied_dot(&brick.dots[3].with_original_dot(pos))
    }
    fn dot_in_board(dot: &Dot) -> bool {
        //0 <= dot.0 && dot.0 < BOARD_X && 0 <= dot.1 && dot.1 < BOARD_Y
        //BUG: should we compare Y ?
        0 <= dot.0 && dot.0 < BOARD_X && 0 <= dot.1
    }
    fn brick_in_board(brick: &BrickView, pos: &Dot) -> bool {
        Self::dot_in_board(&brick.dots[0].with_original_dot(pos))
            && Self::dot_in_board(&brick.dots[1].with_original_dot(pos))
            && Self::dot_in_board(&brick.dots[2].with_original_dot(pos))
            && Self::dot_in_board(&brick.dots[3].with_original_dot(pos))
    }
    pub fn valid_brick_view(&self, brick: &BrickView, pos: &Dot) -> bool {
        Self::brick_in_board(brick, pos) && !self.conflict_brick(brick, pos)
    }
    pub fn valid_brick(&self, brick: &Brick, pos: &Dot) -> bool {
        self.valid_brick_view(&(*brick).into(), pos)
    }
    pub fn clear(&mut self) {
        for i in 0..BOARD_X_Y {
            self.0[i] = false
        }
    }
    pub fn can_clean_line(&self, y: i8) -> bool {
        assert!(0 <= y);
        assert!(y < BOARD_Y_VALID);
        self.0[Self::index(&Dot(0, y))..Self::index(&Dot(0, y + 1))]
            .iter()
            .all(|x| *x)
    }
    pub fn get_clean_lines(&self) -> Vec<i8> {
        let mut vec = Vec::with_capacity(4);
        for i in (0..BOARD_Y_VALID).rev() {
            if self.can_clean_line(i) {
                vec.push(i);
            }
        }
        vec
    }
    pub fn clean_lines(&mut self) -> u32 {
        let deleted_lines = self.get_clean_lines();
        let result = deleted_lines.len();
        for line in deleted_lines {
            self.clean_line(line);
        }
        result as u32
    }

    pub fn clean_line(&mut self, y: i8) {
        assert!(0 <= y);
        assert!(y < BOARD_Y_VALID);

        let dst_below = Self::index(&Dot(0, y));
        let src_below = Self::index(&Dot(0, y + 1));
        let src_high = Self::index(&Dot(0, BOARD_Y));

        //step 1.copy from tail
        self.0.copy_within(src_below..src_high, dst_below);
        //step 2.set last line as false
        self.0[Self::index(&Dot(0, BOARD_Y - 1))..Self::index(&Dot(0, BOARD_Y))]
            .iter_mut()
            .for_each(|x| *x = false);
    }
    // pub fn game_over(&self) -> bool {
    //     self.0[Self::index(&Dot(0, BOARD_Y_VALIDE))..]
    //         .iter()
    //         .any(|x| *x)
    // }
}
