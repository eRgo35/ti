pub struct Timer {
    hours: u64,
    minutes: u64,
    seconds: u64,
    countdown: u64,
    paused: bool,
}

impl Timer {
    pub fn new(hours: u64, minutes: u64, seconds: u64) -> Self {
        Self {
            hours,
            minutes,
            seconds,
            countdown: Self::time_to_countdown(hours, minutes, seconds),
            paused: false,
        }
    }

    pub fn from_cache(cached_countdown: u64) -> Self {
        let (hours, minutes, seconds) = Self::countdown_to_time(cached_countdown);
        Self::new(hours, minutes, seconds)
    }

    pub fn pause(&mut self) {
        self.paused = true;
    }

    pub fn stop(&mut self) {
        self.paused = true;
        self.countdown = 0;
    }

    #[allow(dead_code)]
    pub fn start(&mut self) {
        self.paused = false;
    }

    #[allow(dead_code)]
    pub fn resume(&mut self) {
        self.start();
    }

    pub fn toggle(&mut self) {
        self.paused = !self.paused;
    }

    // TODO: Preserve the original time when resetting aka cache overwrite
    pub fn reset(&mut self) {
        self.countdown = Self::time_to_countdown(self.hours, self.minutes, self.seconds);
    }

    pub fn tick(&mut self) {
        if !self.paused {
            if self.countdown == 0 {
                return;
            }
            self.countdown -= 1;
        }
    }

    pub fn is_finished(&self) -> bool {
        self.countdown == 0
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn time(&self) -> (u64, u64, u64) {
        Self::countdown_to_time(self.countdown)
    }

    pub fn countdown(&self) -> u64 {
        self.countdown
    }

    pub fn print(&self) -> String {
        let (hours, minutes, seconds) = self.time();
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    }

    fn time_to_countdown(hours: u64, minutes: u64, seconds: u64) -> u64 {
        hours * 3600 + minutes * 60 + seconds
    }

    fn countdown_to_time(seconds: u64) -> (u64, u64, u64) {
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        let seconds = seconds % 60;
        (hours, minutes, seconds)
    }
}
