domain StormNodeA {
    state { count: int }
    goal StopAtHundred {
        predicate count == 100
    }
    transition ping {
        step {
            count = count + 1
            send("StormNodeB", "ping")
        }
    }
}

domain StormNodeB {
    state { count: int }
    transition ping {
        step {
            count = count + 1
            send("StormNodeA", "ping")
        }
    }
}
