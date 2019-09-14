## Running Flows

### Run the fibonacci example flow
From the project root you can run the fibonacci sample flow using:

```cargo run  -- samples/fibonacci```

You should get a fibonacci series output to the terminal, 
followed by an "ERROR" on a panic, that is caused by integer overflow 
when the next number gets too big (don't worry, that's expected)