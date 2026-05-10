domain Bootstrapper {
    state { started: int }
    transition run {
        step {
            send("HighProducer", "produce")
            send("LowProducer", "produce")
        }
    }
}

domain HighProducer {
    state { count: int }
    transition produce {
        step {
            count = count + 1
            send_high("Consumer", "task_high")
            send("HighProducer", "produce")
        }
    }
}

domain LowProducer {
    state { count: int }
    transition produce {
        step {
            count = count + 1
            send("Consumer", "task_low")
            send("LowProducer", "produce")
        }
    }
}

domain Consumer {
    state {
        high_processed: int
        low_processed: int
    }
    transition task_high {
        step {
            high_processed = high_processed + 1
        }
    }
    transition task_low {
        step {
            low_processed = low_processed + 1
        }
    }
}
