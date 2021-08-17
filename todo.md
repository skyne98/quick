* As for now, sometimes client can send more than once before server receives, if they run in different contexts (different threads or processes)
  Which means a packet gets lost. Message exchanging needs some ack systems, similar to nng. For example, the first one to implement will be
  Req/Rep. Client will wait for a response after the request is sent.