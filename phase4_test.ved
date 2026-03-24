domain Producer {
    state {
        val: int
    }
    transition send_ping {
        slice step {
            val = val + 1
            if val < 5 {
                send Producer send_ping
            }
        }
    }
}
