domain Producer {
    state {
        sent: int
    }
    
    transition send_pings {
        slice step {
            sent = sent + 1
            send Consumer receive_ping
            send Consumer receive_ping
        }
    }
}

domain Consumer {
    state {
        pings: int
    }
    
    goal high_volume {
        target pings >= 4
    }
    
    transition receive_ping {
        slice step {
            pings = pings + 1
            if pings == 1 {
                send Producer send_pings
            }
        }
    }
}
