domain Producer {
    state {
        sent: int
    }
    transition send_ping {
        slice step {
            sent = 1
            send("Consumer", "receive_ping")
        }
    }
}

domain Consumer {
    state {
        pings: int
    }
    transition receive_ping {
        slice step {
            pings = pings + 1
        }
    }
}
