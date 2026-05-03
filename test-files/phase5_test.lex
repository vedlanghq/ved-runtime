domain Counter {
    state {
        val: int
    }
    goal is_ten {
        target val == 10
    }
    transition increment {
        slice step {
            val = val + 1
            if val < 10 {
                send Counter increment
            }
        }
    }
}
