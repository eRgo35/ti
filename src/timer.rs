pub struct Timer {
    hours: u64,
    minutes: u64,
    seconds: u64,
    countdown: u64,
    original_countdown: u64,
    paused: bool,
    stopped: bool,
}

impl Timer {
    pub fn new(hours: u64, minutes: u64, seconds: u64) -> Self {
        Self {
            hours,
            minutes,
            seconds,
            countdown: Self::time_to_countdown(hours, minutes, seconds),
            original_countdown: Self::time_to_countdown(hours, minutes, seconds),
            paused: false,
            stopped: false,
        }
    }

    pub fn from_cache(
        cached_countdown: u64,
        args_hours: u64,
        args_minutes: u64,
        args_seconds: u64,
    ) -> Self {
        if cached_countdown == 0 {
            return Self::new(args_hours, args_minutes, args_seconds);
        }

        let (hours, minutes, seconds) = Self::countdown_to_time(cached_countdown);

        Self {
            hours,
            minutes,
            seconds,
            countdown: cached_countdown,
            original_countdown: Self::time_to_countdown(args_hours, args_minutes, args_seconds),
            paused: false,
            stopped: false,
        }
    }

    pub fn pause(&mut self) {
        self.paused = true;
    }

    pub fn stop(&mut self) {
        self.stopped = true;
        // self.countdown = 0;
    }

    pub fn start(&mut self) {
        self.paused = false;
    }

    #[allow(dead_code)]
    pub fn resume(&mut self) {
        self.start();
    }

    pub fn toggle(&mut self) {
        if self.countdown == 0 {
            return;
        }

        self.paused = !self.paused;
    }

    pub fn reset(&mut self) {
        self.countdown = self.original_countdown;
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

    pub fn is_stopped(&self) -> bool {
        self.stopped
    }

    #[allow(dead_code)]
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
