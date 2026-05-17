/// Аниме-тянка спрайты (заглушка)
pub struct SpriteAnimator;

impl SpriteAnimator {
    pub fn new() -> Self {
        Self
    }

    pub fn set_mouth_state(&mut self, _state: MouthState) {}
}

pub enum MouthState {
    Closed,
    SlightlyOpen,
    Open,
    Wide,
}
