domain World {
    state {
        tick: int
    }
    
    transition step_time {
        slice step {
            tick = tick + 1
            if tick < 10 {
                send World step_time
            }
        }
    }
}
